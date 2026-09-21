//! Installing and updating a page from its address (`docs/reference/updater.md`).
//!
//! The hosting layout, written by `vpppack --publish`:
//!
//! ```text
//! <site>/<page>.vpp          the full package
//! <site>/<page>.vppm         its signed manifest, for update checks
//! <site>/res/<sha256 hex>    resources, shared by every page of the site
//! ```
//!
//! On every visit the updater fetches the page's manifest, verifies it,
//! compares it with the installed copy, downloads only the resources whose
//! hashes are not already stored, and installs. If the server cannot be
//! reached, the installed copy runs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;
use vpp_format::{Package, PackageError, ResourceHash, SignatureStatus};

use crate::fetch::{Fetch, FetchError, fetch_url};
use crate::store::{Store, write_atomic};
use crate::version::compare_versions;

/// What a sync did, and where the page to run is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResult {
    /// The assembled `.vpp` to run.
    pub package_path: PathBuf,
    /// The page's site id.
    pub site_id: String,
    /// The page name.
    pub page: String,
    /// The site version installed.
    pub version: String,
    /// The server was unreachable, so this is the installed copy.
    pub offline: bool,
    /// A new version was installed.
    pub updated: bool,
    /// How many resources were downloaded.
    pub downloaded: usize,
    /// How many bytes they held.
    pub downloaded_bytes: u64,
}

/// Why a page could not be installed or updated.
#[derive(Debug, Error)]
pub enum SyncError {
    /// Neither the manifest nor the package could be fetched, and no copy is installed.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// What the server sent is not a valid package or manifest.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// The server's page is not signed. Only signed pages are installed from the network.
    #[error("the page at {0} is unsigned; only signed pages are installed from the network")]
    Unsigned(String),
    /// The signature does not match: the page was changed after signing, or is forged.
    #[error("the signature of the page at {0} is invalid")]
    InvalidSignature(String),
    /// The installed page is signed by a different publisher.
    #[error("publisher key changed for {0}; refusing the update")]
    PublisherChanged(String),
    /// The server offers an older version than the one installed.
    #[error("rollback refused: the server offers {offered} but {installed} is installed")]
    Rollback {
        /// The version on the server.
        offered: String,
        /// The version installed.
        installed: String,
    },
    /// A downloaded resource does not match its size or hash.
    #[error("downloaded {0} does not match its hash")]
    ResourceMismatch(String),
    /// The store could not be written.
    #[error("cannot write {}: {source}", path.display())]
    Io {
        /// The file involved.
        path: PathBuf,
        /// What went wrong.
        source: std::io::Error,
    },
}

impl Store {
    /// Installs or updates the page at `address` (a `.vpp` or `.vppm` URL, or
    /// `vpp://`), reporting each step to `log`.
    pub fn sync(
        &self,
        address: &str,
        fetch: &dyn Fetch,
        log: &mut dyn FnMut(&str),
    ) -> Result<SyncResult, SyncError> {
        let url = fetch_url(address);
        let (manifest_url, package_url) = page_urls(&url);

        // 1. The manifest; or the whole package; or, offline, the installed copy.
        let mut full: Option<Package> = None;
        let remote = match fetch.get(&manifest_url) {
            Ok(bytes) => Package::open_manifest(bytes)?,
            Err(manifest_error) => match fetch.get(&package_url) {
                Ok(bytes) => {
                    let package = Package::open(bytes)?;
                    let manifest = Package::open_manifest(package.manifest_bytes().to_vec())?;
                    full = Some(package);
                    log("no manifest served; downloaded the whole page package");
                    manifest
                }
                Err(_) => {
                    return match self.installed_copy(&[address, &url, &manifest_url, &package_url])
                    {
                        Some(result) => {
                            log(&format!(
                                "offline: {manifest_error}; running the installed copy"
                            ));
                            Ok(result)
                        }
                        None => Err(manifest_error.into()),
                    };
                }
            },
        };

        // 2. Only signed pages are ever installed from the network.
        match remote.verify_signature() {
            SignatureStatus::Valid => {}
            SignatureStatus::Unsigned => return Err(SyncError::Unsigned(url)),
            SignatureStatus::Invalid => return Err(SyncError::InvalidSignature(url)),
        }
        let manifest = remote.manifest().clone();
        let page = if manifest.page.is_empty() {
            "index".to_owned()
        } else {
            manifest.page.clone()
        };
        let package_path = self.page_file(&manifest.site_id, &page, "vpp");
        let manifest_path = self.page_file(&manifest.site_id, &page, "vppm");
        let mut result = SyncResult {
            package_path: package_path.clone(),
            site_id: manifest.site_id.clone(),
            page: manifest.page.clone(),
            version: manifest.site_version.clone(),
            offline: false,
            updated: false,
            downloaded: 0,
            downloaded_bytes: 0,
        };

        // 3. The installed copy: same publisher, and no going back.
        let installed_bytes = fs::read(&manifest_path).ok();
        let installed = installed_bytes
            .clone()
            .and_then(|bytes| Package::open_manifest(bytes).ok());
        if let Some(installed) = &installed {
            if installed.publisher_key() != remote.publisher_key() {
                return Err(SyncError::PublisherChanged(manifest.site_id));
            }
            let installed_version = &installed.manifest().site_version;
            if compare_versions(&manifest.site_version, installed_version).is_lt() {
                return Err(SyncError::Rollback {
                    offered: manifest.site_version,
                    installed: installed_version.clone(),
                });
            }
            let unchanged = installed_bytes.as_deref() == Some(remote.manifest_bytes());
            if unchanged && package_path.is_file() {
                log(&format!(
                    "up to date: {} / {page} {}",
                    manifest.site_name, manifest.site_version
                ));
                return Ok(result);
            }
        }

        // 4. Resources: from the store, the downloaded package, or the server.
        let base = base_url(&url);
        let mut resources = BTreeMap::new();
        for entry in remote.entries() {
            let stored_path = self.resource_file(&manifest.site_id, &entry.hash);
            if let Some(bytes) = read_verified(&stored_path, entry.size, &entry.hash) {
                log(&format!("cached: {} ({} bytes)", entry.name, bytes.len()));
                resources.insert(entry.name.clone(), bytes);
                continue;
            }
            let bytes = match &full {
                Some(package) => package.read(&entry.name)?.to_vec(),
                None => {
                    let bytes = fetch.get(&format!("{base}res/{}", entry.hash))?;
                    if bytes.len() as u64 != entry.size || ResourceHash::of(&bytes) != entry.hash {
                        return Err(SyncError::ResourceMismatch(entry.name.clone()));
                    }
                    bytes
                }
            };
            write(&stored_path, &bytes)?;
            log(&format!(
                "downloaded: {} ({} bytes)",
                entry.name,
                bytes.len()
            ));
            result.downloaded += 1;
            result.downloaded_bytes += bytes.len() as u64;
            resources.insert(entry.name.clone(), bytes);
        }

        // 5. Assemble and install. The manifest goes last, so an interrupted
        //    update leaves the previous version runnable.
        let assembled = Package::assemble(&remote, &resources)?;
        write(&package_path, &assembled)?;
        write(&manifest_path, remote.manifest_bytes())?;
        write(
            &self.page_file(&manifest.site_id, &page, "url"),
            address.as_bytes(),
        )?;

        result.updated = true;
        let verb = if installed.is_some() {
            "updated"
        } else {
            "installed"
        };
        log(&format!(
            "{verb}: {} / {page} {} ({} resources, {} bytes downloaded)",
            manifest.site_name, manifest.site_version, result.downloaded, result.downloaded_bytes
        ));
        Ok(result)
    }

    /// The installed page that came from one of `addresses`, if any.
    fn installed_copy(&self, addresses: &[&str]) -> Option<SyncResult> {
        for url_file in self.url_files() {
            let Ok(from) = fs::read_to_string(&url_file) else {
                continue;
            };
            if !addresses.contains(&from.as_str()) {
                continue;
            }
            let package_path = url_file.with_extension("vpp");
            if !package_path.is_file() {
                continue;
            }
            let manifest = fs::read(url_file.with_extension("vppm"))
                .ok()
                .and_then(|bytes| Package::open_manifest(bytes).ok())
                .map(|m| m.manifest().clone())
                .unwrap_or_default();
            return Some(SyncResult {
                package_path,
                site_id: manifest.site_id,
                page: manifest.page,
                version: manifest.site_version,
                offline: true,
                updated: false,
                downloaded: 0,
                downloaded_bytes: 0,
            });
        }
        None
    }
}

/// The manifest and package URLs for a page URL naming either one.
fn page_urls(url: &str) -> (String, String) {
    if let Some(stem) = url.strip_suffix(".vppm") {
        (url.to_owned(), format!("{stem}.vpp"))
    } else if let Some(stem) = url.strip_suffix(".vpp") {
        (format!("{stem}.vppm"), url.to_owned())
    } else {
        (url.to_owned(), url.to_owned())
    }
}

/// Everything up to and including the last `/`: where `res/` is.
fn base_url(url: &str) -> String {
    match url.rfind('/') {
        Some(slash) => url[..=slash].to_owned(),
        None => format!("{url}/"),
    }
}

/// A stored resource, if it is there and still matches its size and hash.
fn read_verified(path: &Path, size: u64, hash: &ResourceHash) -> Option<Vec<u8>> {
    let bytes = fs::read(path).ok()?;
    (bytes.len() as u64 == size && ResourceHash::of(&bytes) == *hash).then_some(bytes)
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), SyncError> {
    write_atomic(path, bytes).map_err(|source| SyncError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::fake::FakeServer;
    use crate::test_dir::TestDir;
    use vpp_format::{Manifest, SigningKey};

    const SITE: &str = "https://example.com/site/";

    fn key() -> SigningKey {
        SigningKey::from_seed([3; 32])
    }

    fn manifest(version: &str) -> Manifest {
        Manifest {
            site_id: "org.example".into(),
            site_name: "Example".into(),
            site_version: version.into(),
            page: "home".into(),
            window: String::new(),
        }
    }

    /// Publishes a version of the home page to `server`, as `vpppack --publish` does.
    fn publish(
        server: &mut FakeServer,
        version: &str,
        resources: &[(&str, &[u8])],
        key: Option<&SigningKey>,
    ) -> Vec<u8> {
        let resources: BTreeMap<String, Vec<u8>> = resources
            .iter()
            .map(|(n, b)| (n.to_string(), b.to_vec()))
            .collect();
        let bytes = Package::build(&manifest(version), &resources, key);
        let package = Package::open(bytes.clone()).unwrap();
        server
            .files
            .insert(format!("{SITE}home.vpp"), bytes.clone());
        server.files.insert(
            format!("{SITE}home.vppm"),
            package.manifest_bytes().to_vec(),
        );
        for entry in package.entries() {
            let data = package.read(&entry.name).unwrap().to_vec();
            server
                .files
                .insert(format!("{SITE}res/{}", entry.hash), data);
        }
        bytes
    }

    fn sync(store: &Store, server: &FakeServer) -> Result<SyncResult, SyncError> {
        store.sync(&format!("{SITE}home.vpp"), server, &mut |_| {})
    }

    fn resource_requests(server: &FakeServer) -> usize {
        let count = server
            .requests
            .borrow()
            .iter()
            .filter(|u| u.contains("/res/"))
            .count();
        server.requests.borrow_mut().clear();
        count
    }

    #[test]
    fn installs_a_page_byte_for_byte() {
        let dir = TestDir::new("install");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        let published = publish(
            &mut server,
            "1.0.0",
            &[("dom.bin", b"dom"), ("style/00-a.bin", b"css")],
            Some(&key()),
        );

        let result = sync(&store, &server).unwrap();
        assert!(result.updated && !result.offline);
        assert_eq!((result.downloaded, result.downloaded_bytes), (2, 6));
        assert_eq!(result.site_id, "org.example");
        assert_eq!(fs::read(&result.package_path).unwrap(), published);
    }

    #[test]
    fn a_second_visit_with_no_change_downloads_nothing() {
        let dir = TestDir::new("unchanged");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        sync(&store, &server).unwrap();
        resource_requests(&server);

        let result = sync(&store, &server).unwrap();
        assert!(!result.updated);
        assert_eq!(resource_requests(&server), 0);
    }

    #[test]
    fn an_update_downloads_only_changed_resources() {
        let dir = TestDir::new("update");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(
            &mut server,
            "1.0.0",
            &[("dom.bin", b"dom"), ("style/00-a.bin", b"old css")],
            Some(&key()),
        );
        sync(&store, &server).unwrap();
        resource_requests(&server);

        let v2 = publish(
            &mut server,
            "1.1.0",
            &[("dom.bin", b"dom"), ("style/00-a.bin", b"new css")],
            Some(&key()),
        );
        let result = sync(&store, &server).unwrap();
        assert!(result.updated);
        assert_eq!(result.downloaded, 1);
        assert_eq!(resource_requests(&server), 1);
        assert_eq!(fs::read(&result.package_path).unwrap(), v2);
    }

    #[test]
    fn rollback_is_refused() {
        let dir = TestDir::new("rollback");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "2.0.0", &[("dom.bin", b"new")], Some(&key()));
        sync(&store, &server).unwrap();

        publish(&mut server, "1.9.0", &[("dom.bin", b"old")], Some(&key()));
        let err = sync(&store, &server).unwrap_err();
        assert!(matches!(err, SyncError::Rollback { .. }), "{err}");
    }

    #[test]
    fn a_different_publisher_is_refused() {
        let dir = TestDir::new("publisher");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        sync(&store, &server).unwrap();

        // INTENTIONAL: an impostor re-signs a newer version of the same site with its own key.
        let impostor = SigningKey::from_seed([66; 32]);
        publish(
            &mut server,
            "9.0.0",
            &[("dom.bin", b"evil")],
            Some(&impostor),
        );
        let err = sync(&store, &server).unwrap_err();
        assert!(matches!(err, SyncError::PublisherChanged(_)), "{err}");
    }

    #[test]
    fn unsigned_or_tampered_pages_are_refused() {
        let dir = TestDir::new("unsigned");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], None);
        assert!(matches!(
            sync(&store, &server).unwrap_err(),
            SyncError::Unsigned(_)
        ));

        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        let manifest_url = format!("{SITE}home.vppm");
        let mut tampered = server.files[&manifest_url].clone();
        tampered[10] ^= 1; // inside the site id, in the signed region
        server.files.insert(manifest_url, tampered);
        assert!(matches!(
            sync(&store, &server).unwrap_err(),
            SyncError::InvalidSignature(_)
        ));
    }

    #[test]
    fn a_resource_that_does_not_match_its_hash_is_refused() {
        let dir = TestDir::new("bad-resource");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        let res = server
            .files
            .keys()
            .find(|u| u.contains("/res/"))
            .unwrap()
            .clone();
        server.files.insert(res, b"not the dom".to_vec());
        assert!(matches!(
            sync(&store, &server).unwrap_err(),
            SyncError::ResourceMismatch(_)
        ));
    }

    #[test]
    fn without_a_manifest_the_whole_package_is_used() {
        let dir = TestDir::new("no-manifest");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        let published = publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        server.files.retain(|u, _| u.ends_with(".vpp"));

        let result = sync(&store, &server).unwrap();
        assert_eq!(fs::read(&result.package_path).unwrap(), published);
        assert_eq!(resource_requests(&server), 0);
    }

    #[test]
    fn offline_runs_the_installed_copy() {
        let dir = TestDir::new("offline");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        let installed = sync(&store, &server).unwrap();

        server.offline = true;
        let mut lines = Vec::new();
        let result = store
            .sync(&format!("{SITE}home.vpp"), &server, &mut |line| {
                lines.push(line.to_owned())
            })
            .unwrap();
        assert!(result.offline);
        assert_eq!(result.package_path, installed.package_path);
        assert_eq!(result.version, "1.0.0");
        assert!(lines[0].starts_with("offline"));

        // A page never installed has nothing to fall back on.
        let err = store
            .sync("https://other.example/x.vpp", &server, &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, SyncError::Fetch(_)));
    }

    #[test]
    fn vpp_addresses_fetch_over_https() {
        let dir = TestDir::new("vpp-scheme");
        let store = Store::new(dir.path());
        let mut server = FakeServer::default();
        publish(&mut server, "1.0.0", &[("dom.bin", b"dom")], Some(&key()));
        store
            .sync("vpp://example.com/site/home.vpp", &server, &mut |_| {})
            .unwrap();
        assert!(
            server
                .requests
                .borrow()
                .iter()
                .all(|u| u.starts_with("https://"))
        );
    }

    #[test]
    fn page_and_base_urls() {
        assert_eq!(
            page_urls("https://h/s/p.vpp"),
            ("https://h/s/p.vppm".into(), "https://h/s/p.vpp".into())
        );
        assert_eq!(
            page_urls("https://h/s/p.vppm"),
            ("https://h/s/p.vppm".into(), "https://h/s/p.vpp".into())
        );
        assert_eq!(base_url("https://h/s/p.vpp"), "https://h/s/");
    }
}

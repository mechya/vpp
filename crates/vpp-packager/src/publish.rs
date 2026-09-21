//! `vpppack <project>`: one signed `.vpp` per compiled page, and with
//! `--publish` the hosting layout any static server can serve
//! (`docs/reference/packager.md`, `docs/reference/updater.md`):
//!
//! ```text
//! out/
//! ├── home.vpp, about.vpp     one package per page
//! ├── home.vppm, about.vppm   each page's manifest, for update checks
//! └── res/<sha256>            every distinct resource once
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vpp_format::{Package, SigningKey, SiteConfig};

/// What to package, from the command line.
pub(crate) struct PackOptions<'a> {
    pub(crate) project: &'a Path,
    pub(crate) key: Option<&'a Path>,
    pub(crate) output: Option<&'a Path>,
    pub(crate) publish: bool,
}

/// Packages every page under `<project>/dist/`.
pub(crate) fn pack(options: &PackOptions<'_>, out: &mut dyn Write) -> Result<()> {
    let project = options.project;
    let config_path = project.join("vpp.json");
    let config_text = fs::read_to_string(&config_path).with_context(|| {
        format!(
            "cannot read {}\n  create it with at least:  {{ \"id\": \"com.example.site\", \"name\": \"Site\", \"version\": \"1.0.0\" }}",
            config_path.display()
        )
    })?;
    let config = SiteConfig::parse(&config_text)?;

    let key = match options.key {
        Some(path) => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("cannot read key file {}", path.display()))?;
            Some(SigningKey::from_file_text(&text).with_context(|| path.display().to_string())?)
        }
        None => None,
    };
    if options.publish && key.is_none() {
        bail!(
            "--publish requires --key; the viewer never installs unsigned pages from the network"
        );
    }

    let dist = project.join("dist");
    let pages = compiled_pages(&dist)?;
    let out_dir = options
        .output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project.join("out"));

    let plural = if pages.len() == 1 { "" } else { "s" };
    writeln!(
        out,
        "vpppack: {} {} ({}), {} page{plural}",
        config.name,
        config.version,
        config.id,
        pages.len()
    )?;
    match &key {
        Some(key) => writeln!(out, "  signed by  {}", key.public_key())?,
        None => writeln!(
            out,
            "  UNSIGNED: the viewer will refuse these pages unless run with --allow-unsigned"
        )?,
    }

    let mut published = 0;
    for (page, page_dist) in &pages {
        let resources = collect_resources(page_dist)?;
        let bytes = Package::build(&config.manifest(page), &resources, key.as_ref());
        let vpp_path = out_dir.join(format!("{page}.vpp"));
        write_file(&vpp_path, &bytes)?;
        writeln!(
            out,
            "  {:<14} {} resources, {} bytes -> {}",
            format!("{page}.vpp"),
            resources.len(),
            bytes.len(),
            vpp_path.display()
        )?;

        if options.publish {
            let package = Package::open(bytes)?;
            write_file(
                &out_dir.join(format!("{page}.vppm")),
                package.manifest_bytes(),
            )?;
            for entry in package.entries() {
                let res_path = out_dir.join("res").join(entry.hash.to_string());
                // Named by hash: an existing file already has these exact bytes.
                if res_path.exists() {
                    continue;
                }
                write_file(&res_path, package.read(&entry.name)?)?;
                published += 1;
            }
        }
    }

    if options.publish {
        writeln!(
            out,
            "  published  {}: {} manifests, {published} distinct resources in res/",
            out_dir.display(),
            pages.len()
        )?;
        writeln!(
            out,
            "             serve this directory and open a page's .vpp URL in the viewer"
        )?;
    }
    Ok(())
}

/// Each `dist/<page>/` that contains a `dom.bin`, sorted by page name. A
/// single `dist/dom.bin`, from a one-page site, is the page `index`.
fn compiled_pages(dist: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut pages = Vec::new();
    if let Ok(entries) = fs::read_dir(dist) {
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() && path.join("dom.bin").is_file() {
                let name = utf8_name(&path)?;
                pages.push((name, path));
            }
        }
    }
    pages.sort();
    if pages.is_empty() && dist.join("dom.bin").is_file() {
        pages.push(("index".to_owned(), dist.to_path_buf()));
    }
    if pages.is_empty() {
        bail!(
            "{} has no compiled pages; run vppc on the project first",
            dist.display()
        );
    }
    Ok(pages)
}

/// Every file under `page_dist`, named by its path relative to it with `/` separators.
fn collect_resources(page_dist: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut resources = BTreeMap::new();
    let mut folders = vec![page_dist.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let entries =
            fs::read_dir(&folder).with_context(|| format!("cannot read {}", folder.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.is_file() {
                let relative = path.strip_prefix(page_dist).expect("found under page_dist");
                let mut name = Vec::new();
                for part in relative.components() {
                    let part = part
                        .as_os_str()
                        .to_str()
                        .with_context(|| format!("{} is not a UTF-8 file name", path.display()))?;
                    name.push(part);
                }
                let bytes =
                    fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
                resources.insert(name.join("/"), bytes);
            }
        }
    }
    Ok(resources)
}

fn utf8_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .with_context(|| format!("{} is not a UTF-8 file name", path.display()))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;
    use vpp_format::{ResourceHash, SignatureStatus};

    const CONFIG: &str = r#"{ "id": "org.vpp.test", "name": "Test", "version": "1.2.0" }"#;

    /// A project with two compiled pages that share a stylesheet.
    fn project() -> TestDir {
        let dir = TestDir::new("project");
        dir.write("vpp.json", CONFIG.as_bytes());
        dir.write("dist/home/dom.bin", b"home dom");
        dir.write("dist/home/style/00-global.bin", b"shared style");
        dir.write("dist/about/dom.bin", b"about dom");
        dir.write("dist/about/style/00-global.bin", b"shared style");
        dir.write("dist/not-a-page/readme.txt", b"no dom.bin here");
        dir.write(
            "publisher.key",
            SigningKey::from_seed([5; 32]).to_file_text().as_bytes(),
        );
        dir
    }

    fn run(dir: &TestDir, signed: bool, publish: bool) -> Result<String> {
        let key = dir.path().join("publisher.key");
        let options = PackOptions {
            project: dir.path(),
            key: signed.then_some(key.as_path()),
            output: None,
            publish,
        };
        let mut out = Vec::new();
        pack(&options, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn packages_every_compiled_page_signed() {
        let dir = project();
        let log = run(&dir, true, false).unwrap();
        assert!(log.contains("2 pages"));

        let home = Package::open(fs::read(dir.path().join("out/home.vpp")).unwrap()).unwrap();
        assert_eq!(home.verify_signature(), SignatureStatus::Valid);
        assert_eq!(home.manifest().page, "home");
        assert_eq!(home.manifest().site_id, "org.vpp.test");
        assert_eq!(home.read("dom.bin").unwrap(), b"home dom");
        assert_eq!(home.read("style/00-global.bin").unwrap(), b"shared style");
        assert!(dir.path().join("out/about.vpp").is_file());
        assert!(!dir.path().join("out/not-a-page.vpp").exists());
        assert!(!dir.path().join("out/home.vppm").exists());
    }

    #[test]
    fn unsigned_packages_are_marked_and_warned_about() {
        let dir = project();
        let log = run(&dir, false, false).unwrap();
        assert!(log.contains("UNSIGNED"));
        let home = Package::open(fs::read(dir.path().join("out/home.vpp")).unwrap()).unwrap();
        assert_eq!(home.verify_signature(), SignatureStatus::Unsigned);
    }

    #[test]
    fn publish_writes_manifests_and_each_resource_once() {
        let dir = project();
        let log = run(&dir, true, true).unwrap();
        // home dom, about dom, and the shared style: three distinct resources.
        assert!(log.contains("2 manifests, 3 distinct resources"), "{log}");

        let vppm =
            Package::open_manifest(fs::read(dir.path().join("out/home.vppm")).unwrap()).unwrap();
        assert_eq!(vppm.verify_signature(), SignatureStatus::Valid);
        let shared = dir
            .path()
            .join("out/res")
            .join(ResourceHash::of(b"shared style").to_string());
        assert_eq!(fs::read(shared).unwrap(), b"shared style");
        assert_eq!(fs::read_dir(dir.path().join("out/res")).unwrap().count(), 3);
    }

    #[test]
    fn publish_without_a_key_is_refused() {
        let dir = project();
        let err = run(&dir, false, true).unwrap_err();
        assert!(err.to_string().contains("--publish requires --key"));
        assert!(!dir.path().join("out").exists());
    }

    #[test]
    fn one_page_site_is_index() {
        let dir = TestDir::new("one-page");
        dir.write("vpp.json", CONFIG.as_bytes());
        dir.write("dist/dom.bin", b"only page");
        run(&dir, false, false).unwrap();
        let index = Package::open(fs::read(dir.path().join("out/index.vpp")).unwrap()).unwrap();
        assert_eq!(index.manifest().page, "index");
    }

    #[test]
    fn missing_config_or_pages_explain_what_to_do() {
        let dir = TestDir::new("empty");
        let err = run(&dir, false, false).unwrap_err();
        assert!(format!("{err:#}").contains("vpp.json"));

        dir.write("vpp.json", CONFIG.as_bytes());
        let err = run(&dir, false, false).unwrap_err();
        assert!(err.to_string().contains("run vppc"));
    }
}

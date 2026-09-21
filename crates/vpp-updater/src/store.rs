//! The local store of installed pages (`docs/reference/updater.md`):
//!
//! ```text
//! <root>/<site>/pages/<page>.vppm   the installed manifest
//! <root>/<site>/pages/<page>.vpp    the assembled package the viewer runs
//! <root>/<site>/pages/<page>.url    the address it came from
//! <root>/<site>/res/<sha256>        resources, shared by every page and version of the site
//! ```
//!
//! Every file is written under a temporary name and renamed into place, and
//! a page's manifest is replaced last, so an interrupted update leaves the
//! previous version intact and runnable.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use vpp_format::ResourceHash;

/// The folder a store lives in. The viewer chooses it, from `vpp-platform`.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// A store in `root`, which is created when first written to.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The store's folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn site_dir(&self, site_id: &str) -> PathBuf {
        self.root.join(safe_name(site_id))
    }

    pub(crate) fn page_file(&self, site_id: &str, page: &str, extension: &str) -> PathBuf {
        self.site_dir(site_id)
            .join("pages")
            .join(format!("{}.{extension}", safe_name(page)))
    }

    pub(crate) fn resource_file(&self, site_id: &str, hash: &ResourceHash) -> PathBuf {
        self.site_dir(site_id).join("res").join(hash.to_string())
    }

    /// Every `pages/*.url` file in the store, for finding an installed page by address.
    pub(crate) fn url_files(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let Ok(sites) = fs::read_dir(&self.root) else {
            return found;
        };
        for site in sites.flatten() {
            let Ok(pages) = fs::read_dir(site.path().join("pages")) else {
                continue;
            };
            for page in pages.flatten() {
                let path = page.path();
                if path.extension().is_some_and(|e| e == "url") {
                    found.push(path);
                }
            }
        }
        found.sort();
        found
    }
}

/// Writes `bytes` to `path` through a temporary file renamed into place, so
/// readers see either the old file or the new one, never a partial one.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

/// A file or folder name for a site id or page name, safe on every file
/// system, and different for every different input.
///
/// Lower-case letters, digits, `-`, and `.` are kept; every other byte
/// becomes `_` and two upper-case hex digits (`/` becomes `_2F`). The C++
/// updater replaced them with a plain `_`, so two ids could share a folder,
/// and an id of `..` escaped the store. Because upper-case letters are
/// encoded too, the result stays unique on case-insensitive file systems.
/// A leading or trailing `.`, and names Windows reserves (`con`, `nul`,
/// `com1`...), are encoded the same way. Very long names are shortened with a
/// hash, so paths stay within Windows limits.
pub fn safe_name(id: &str) -> String {
    const MAX: usize = 120;
    let bytes = id.as_bytes();
    let mut out = String::with_capacity(id.len());
    for (i, &b) in bytes.iter().enumerate() {
        let edge_dot = b == b'.' && (i == 0 || i == bytes.len() - 1);
        let reserved_start = i == 0 && is_windows_reserved(id);
        let kept = matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.');
        if kept && !edge_dot && !reserved_start {
            out.push(b as char);
        } else {
            out.push_str(&format!("_{b:02X}"));
        }
    }
    if out.is_empty() {
        return "_".to_owned();
    }
    if out.len() > MAX {
        let hash = ResourceHash::of(id.as_bytes()).to_string();
        out.truncate(MAX - 17);
        // The cut must not split an `_XX` escape: drop a trailing partial one.
        while let Some(at) = out.rfind('_').filter(|&at| at + 3 > out.len()) {
            out.truncate(at);
        }
        out.push('-');
        out.push_str(&hash[..16]);
    }
    out
}

/// Whether the part before the first `.` is a device name Windows reserves.
fn is_windows_reserved(id: &str) -> bool {
    let stem = id
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
        || ((stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;

    #[test]
    fn ordinary_ids_stay_readable() {
        assert_eq!(
            safe_name("org.vpp.examples.hello-world"),
            "org.vpp.examples.hello-world"
        );
        assert_eq!(safe_name("home"), "home");
    }

    #[test]
    fn every_different_id_gets_a_different_name() {
        // All of these were "org.vpp_a" or similar in the C++ updater.
        let ids = [
            "org.vpp/a",
            "org.vpp_a",
            "org.vpp a",
            "Org.vpp.a",
            "org.vpp.a",
        ];
        let mut names: Vec<_> = ids
            .iter()
            .map(|id| safe_name(id).to_ascii_lowercase())
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ids.len(), "{names:?}");
    }

    #[test]
    fn nothing_escapes_the_store_or_hits_a_reserved_name() {
        // INTENTIONAL: hostile ids. Each must stay a single ordinary folder name.
        for id in [
            "..",
            ".",
            "../x",
            "a/../../b",
            "..\\x",
            "con",
            "NUL.txt",
            "com1",
            "x.",
        ] {
            let name = safe_name(id);
            assert!(
                !name.contains('/') && !name.contains('\\'),
                "{id} -> {name}"
            );
            assert!(
                !name.starts_with('.') && !name.ends_with('.'),
                "{id} -> {name}"
            );
            assert!(!is_windows_reserved(&name), "{id} -> {name}");
        }
        assert_eq!(safe_name(""), "_");
    }

    #[test]
    fn long_ids_are_shortened_but_stay_distinct() {
        let a = "a".repeat(300);
        let b = format!("{}b", "a".repeat(299));
        assert!(safe_name(&a).len() <= 120);
        assert_ne!(safe_name(&a), safe_name(&b));
        let escaped = "/".repeat(100);
        let name = safe_name(&escaped);
        assert!(name.len() <= 120);
        // No escape is cut in half before the hash.
        let before_hash = name.rsplit_once('-').unwrap().0;
        assert!(before_hash.len().is_multiple_of(3), "{name}");
    }

    #[test]
    fn atomic_write_replaces_the_whole_file() {
        let dir = TestDir::new("atomic");
        let path = dir.path().join("a/b/file");
        write_atomic(&path, b"first version").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!dir.path().join("a/b/file.tmp").exists());
    }
}

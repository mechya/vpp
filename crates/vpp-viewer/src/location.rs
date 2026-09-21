//! What the viewer can open, told apart by the address alone.

use std::path::{Path, PathBuf};

use vpp_updater::is_remote_url;

/// A page address, classified (`docs/reference/viewer.md`, "Current State").
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Location {
    /// No address: the viewer's own start page.
    Start,
    /// An `http://`, `https://`, or `vpp://` address, installed through the store.
    Remote(String),
    /// A compiled `dist/<page>` folder, read without hashes.
    Directory(PathBuf),
    /// A `.vpp` package file.
    Package(PathBuf),
    /// A page's HTML source, expanded and parsed as text.
    Development(PathBuf),
}

impl Location {
    /// Classifies `address`. A path to a `.bin` file means its folder.
    pub(crate) fn parse(address: &str) -> Self {
        if address.is_empty() {
            return Self::Start;
        }
        if is_remote_url(address) {
            return Self::Remote(address.to_owned());
        }
        let path = Path::new(address);
        if path.is_dir() {
            return Self::Directory(path.to_owned());
        }
        match extension(path).as_str() {
            "vpp" => Self::Package(path.to_owned()),
            "bin" => Self::Directory(path.parent().unwrap_or(Path::new("")).to_owned()),
            _ => Self::Development(path.to_owned()),
        }
    }
}

/// The extension of `path`, lower-cased, or empty.
pub(crate) fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_classified() {
        assert_eq!(Location::parse(""), Location::Start);
        assert_eq!(
            Location::parse("vpp://example.com/site/home.vpp"),
            Location::Remote("vpp://example.com/site/home.vpp".into())
        );
        assert_eq!(
            Location::parse("site/Home.VPP"),
            Location::Package("site/Home.VPP".into())
        );
        assert_eq!(
            Location::parse("dist/home/dom.bin"),
            Location::Directory("dist/home".into())
        );
        assert_eq!(
            Location::parse("pages/home.html"),
            Location::Development("pages/home.html".into())
        );
        let here = env!("CARGO_MANIFEST_DIR");
        assert_eq!(Location::parse(here), Location::Directory(here.into()));
    }
}

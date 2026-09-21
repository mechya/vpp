//! The signed description of one page: which site it belongs to, and which page it is.

/// The manifest fields of a package, in the order they are stored
/// (`docs/formats/package.md`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// The site identifier, reverse-DNS style, stable across versions and shared by all pages.
    pub site_id: String,
    /// The site name users see.
    pub site_name: String,
    /// The site version set by the publisher, for example `1.0.0`.
    pub site_version: String,
    /// The page name within the site, for example `home`.
    pub page: String,
    /// The `window` object of `vpp.json` as JSON text, or empty: the site's shell preferences.
    pub window: String,
}

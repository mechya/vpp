//! Loading a page from any [`Location`] into a document, its stylesheets,
//! and its scripts. Nothing here runs a script.
//!
//! A package is checked completely before any of it is used: the signature,
//! the publisher key pinned for the site, and every resource's hash
//! (`docs/reference/viewer.md`, "Release mode"). Any failure becomes an error
//! page, never a partly loaded page.

use std::fs;
use std::path::Path;

use vpp_dom::{Document, decode_dom, parse_html};
use vpp_format::{CodeBin, Package, SignatureStatus, SiteConfig};
use vpp_style::{StyleSheet, decode_stylesheet};
use vpp_template::{TemplateOptions, expand_page, find_project_root};
use vpp_updater::{Fetch, Store, Trust, TrustDecision};

use crate::location::Location;

/// A page ready to start.
pub(crate) struct LoadedPage {
    pub(crate) document: Document,
    /// In cascade order.
    pub(crate) sheets: Vec<StyleSheet>,
    /// In the order they run.
    pub(crate) scripts: Vec<CodeBin>,
    pub(crate) title: String,
    /// The site the page belongs to, or empty for the viewer's own pages.
    pub(crate) site_id: String,
    /// The site's `window` preferences as JSON text, or empty.
    pub(crate) window: String,
}

/// Why a page could not be loaded, shown as an error page.
struct LoadError {
    title: &'static str,
    detail: String,
}

impl LoadError {
    fn new(title: &'static str, detail: impl Into<String>) -> Self {
        Self {
            title,
            detail: detail.into(),
        }
    }

    fn rejected(detail: impl Into<String>) -> Self {
        Self::new("Package rejected", detail)
    }
}

/// What loading needs from the viewer.
pub(crate) struct Loader<'a> {
    pub(crate) trust: &'a Trust,
    pub(crate) store: &'a Store,
    pub(crate) fetch: &'a dyn Fetch,
    /// `--allow-unsigned`: accept unsigned packages, for development only.
    pub(crate) allow_unsigned: bool,
}

impl Loader<'_> {
    /// Loads `location`, or an error page saying why it could not be. What
    /// happened is added to `log`.
    pub(crate) fn load(&self, location: &Location, log: &mut Vec<String>) -> LoadedPage {
        let loaded = match location {
            Location::Start => {
                log.push("mode: start".into());
                Ok(start_page())
            }
            Location::Remote(url) => self.load_remote(url, log),
            Location::Directory(dir) => load_directory(dir, log),
            Location::Package(file) => self.load_package(file, log),
            Location::Development(page) => load_development(page, log),
        };
        loaded.unwrap_or_else(|e| {
            log.push(format!("page error: {}", e.detail));
            error_page(e.title, &e.detail)
        })
    }

    fn load_remote(&self, url: &str, log: &mut Vec<String>) -> Result<LoadedPage, LoadError> {
        log.push(format!("mode: remote\nurl: {url}"));
        let synced = self
            .store
            .sync(url, self.fetch, &mut |line| {
                log.push(format!("update: {line}"))
            })
            .map_err(|e| LoadError::new("Could not fetch page", e.to_string()))?;
        self.load_package(&synced.package_path, log)
    }

    fn load_package(&self, file: &Path, log: &mut Vec<String>) -> Result<LoadedPage, LoadError> {
        log.push("mode: release (package)".into());
        let bytes = fs::read(file).map_err(|e| {
            LoadError::new(
                "Could not open package",
                format!("cannot read {}: {e}", file.display()),
            )
        })?;
        let package =
            Package::open(bytes).map_err(|e| LoadError::new("Invalid package", e.to_string()))?;
        let m = package.manifest();
        log.push(format!(
            "package: {} / {} {} ({}), {} resources",
            m.site_name,
            m.page,
            m.site_version,
            m.site_id,
            package.entries().len()
        ));

        // 1. The signature. Nothing below runs on an unsigned or changed package.
        match package.verify_signature() {
            SignatureStatus::Invalid => {
                return Err(LoadError::rejected(
                    "signature invalid: the package was modified after it was signed",
                ));
            }
            SignatureStatus::Unsigned if !self.allow_unsigned => {
                return Err(LoadError::rejected(
                    "unsigned package. Sign it with vpppack --key, or start the viewer \
                     with --allow-unsigned for development.",
                ));
            }
            SignatureStatus::Unsigned => {
                log.push("warning: unsigned package accepted because of --allow-unsigned".into());
            }
            SignatureStatus::Valid => {
                log.push("signature: valid".into());
                // 2. The publisher key pinned for this site.
                let key = package
                    .publisher_key()
                    .expect("a valid signature has a key");
                match self.trust.check(&m.site_id, key) {
                    Ok(TrustDecision::Known) => log.push("publisher: known".into()),
                    Ok(TrustDecision::FirstUse) => {
                        log.push("publisher: trusted on first use".into());
                    }
                    Err(e) => return Err(LoadError::rejected(e.to_string())),
                }
            }
        }

        // 3. Every resource's hash, so one bad resource rejects the whole package.
        let mut names = Vec::new();
        for entry in package.entries() {
            package
                .read(&entry.name)
                .map_err(|e| LoadError::rejected(e.to_string()))?;
            names.push(entry.name.clone());
        }
        names.sort();
        log.push(format!("verified: {} resources", names.len()));

        let fallback_title = if m.page.is_empty() {
            m.site_name.clone()
        } else {
            format!("{} / {}", m.site_name, m.page)
        };
        let mut page = from_resources(&names, |name| package.read(name).map(<[u8]>::to_vec), log)?;
        page.title = title_of(&page.document).unwrap_or(fallback_title);
        page.site_id = m.site_id.clone();
        page.window = m.window.clone();
        Ok(page)
    }
}

/// A compiled `dist/<page>` folder, read straight from disk without hashes.
fn load_directory(dir: &Path, log: &mut Vec<String>) -> Result<LoadedPage, LoadError> {
    log.push("mode: release (directory)".into());
    let mut names = Vec::new();
    list_files(dir, dir, &mut names);
    names.sort();
    let mut page = from_resources(&names, |name| fs::read(dir.join(name)), log)?;
    let folder = dir.file_name().unwrap_or_default().to_string_lossy();
    page.title = title_of(&page.document).unwrap_or_else(|| folder.into_owned());
    Ok(page)
}

/// Every file under `dir`, as `/`-separated paths relative to `root`.
fn list_files(root: &Path, dir: &Path, names: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            list_files(root, &path, names);
        } else if let Ok(relative) = path.strip_prefix(root) {
            names.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Builds a page from compiled resources: `dom.bin`, then `style/*.bin` and
/// `code/*.bin` in name order. `names` is sorted; `read` supplies each resource.
fn from_resources<E: std::fmt::Display>(
    names: &[String],
    read: impl Fn(&str) -> Result<Vec<u8>, E>,
    log: &mut Vec<String>,
) -> Result<LoadedPage, LoadError> {
    let bytes =
        read("dom.bin").map_err(|e| LoadError::new("Could not open page", e.to_string()))?;
    let document = decode_dom(&bytes).map_err(|e| LoadError::new("Invalid page", e.to_string()))?;
    log.push(format!("page: dom.bin, {} bytes", bytes.len()));

    let mut sheets = Vec::new();
    for name in names
        .iter()
        .filter(|n| n.starts_with("style/") || *n == "style.bin")
    {
        match read(name)
            .map_err(|e| e.to_string())
            .and_then(|bytes| decode_stylesheet(&bytes).map_err(|e| e.to_string()))
        {
            Ok(sheet) => {
                log.push(format!("style: {name} ({} rules)", sheet.rules.len()));
                sheets.push(sheet);
            }
            Err(e) => log.push(format!("style error: {name}: {e}")),
        }
    }

    let mut scripts = Vec::new();
    for name in names
        .iter()
        .filter(|n| n.starts_with("code/") || *n == "code.bin")
    {
        match read(name)
            .map_err(|e| e.to_string())
            .and_then(|bytes| CodeBin::decode(&bytes).map_err(|e| e.to_string()))
        {
            Ok(script) => scripts.push(script),
            Err(e) => log.push(format!("script error: {name}: {e}")),
        }
    }

    Ok(LoadedPage {
        document,
        sheets,
        scripts,
        title: String::new(),
        site_id: String::new(),
        window: String::new(),
    })
}

/// A page's HTML source, expanded with its templates, everything parsed as text.
fn load_development(page: &Path, log: &mut Vec<String>) -> Result<LoadedPage, LoadError> {
    log.push(format!("mode: development\npage: {}", page.display()));
    let root = find_project_root(page);
    let mut options = TemplateOptions {
        project_root: root.clone(),
        ..TemplateOptions::default()
    };
    // A site without an id in its vpp.json is still told apart by its folder.
    let mut site_id = format!("dev:{}", root.display());
    let mut window = String::new();
    if let Ok(json) = fs::read_to_string(root.join("vpp.json")) {
        match SiteConfig::parse(&json) {
            Ok(config) => {
                options.component_aliases = config.components;
                site_id = config.id;
                window = config.window;
            }
            Err(e) => log.push(format!("warning: vpp.json: {e}")),
        }
    }

    let expanded = expand_page(page, &options)
        .map_err(|e| LoadError::new("Could not build page", e.to_string()))?;
    log.extend(expanded.warnings.iter().map(|w| format!("warning: {w}")));

    let sheets = expanded
        .styles
        .into_iter()
        .map(|style| {
            log.push(format!(
                "style: {} ({} rules)",
                style.name,
                style.sheet.rules.len()
            ));
            style.sheet
        })
        .collect();
    let scripts = expanded
        .scripts
        .into_iter()
        .map(|script| {
            let filename = script
                .path
                .as_deref()
                .and_then(Path::file_name)
                .map_or(script.name, |f| f.to_string_lossy().into_owned());
            CodeBin::classic(filename, script.source)
        })
        .collect();
    let stem = page.file_stem().unwrap_or_default().to_string_lossy();
    Ok(LoadedPage {
        title: title_of(&expanded.document).unwrap_or_else(|| stem.into_owned()),
        document: expanded.document,
        sheets,
        scripts,
        site_id,
        window,
    })
}

/// The text of the page's `<title>`, if it has a non-empty one.
fn title_of(document: &Document) -> Option<String> {
    let title = document.find_first(document.root(), "title")?;
    Some(document.text_content(title)).filter(|t| !t.is_empty())
}

/// The viewer's own page, shown when it is opened without an address.
fn start_page() -> LoadedPage {
    own_page(
        "VPP Viewer",
        parse_html(
            "<html><body><h1>VPP Viewer</h1>\
             <p>Enter the address of a page in the bar above: a .vpp URL such as \
             https://example.com/site/home.vpp, or a local .vpp file or page.html.</p>\
             <p>Press Ctrl+L to edit the address at any time. Alt+Left and Alt+Right \
             move through history.</p></body></html>",
        ),
    )
}

/// A page saying what went wrong. The detail is set as text, never parsed,
/// so an error message cannot inject markup.
fn error_page(title: &str, detail: &str) -> LoadedPage {
    let mut document = parse_html("<html><body><h1 id=title></h1><p id=detail></p></body></html>");
    for (id, text) in [("title", title), ("detail", detail)] {
        let node = document
            .find_by_id(document.root(), id)
            .expect("the error page has both elements");
        document.set_text_content(node, text);
    }
    own_page(title, document)
}

fn own_page(title: &str, document: Document) -> LoadedPage {
    LoadedPage {
        document,
        sheets: Vec::new(),
        scripts: Vec::new(),
        title: title.to_owned(),
        site_id: String::new(),
        window: String::new(),
    }
}

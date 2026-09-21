//! History, and where a link leads.

use std::path::Path;

use vpp_updater::is_remote_url;

use crate::location::extension;

/// The addresses visited in this window, and which one is showing.
#[derive(Debug)]
pub(crate) struct History {
    entries: Vec<String>,
    index: usize,
}

impl History {
    /// A history holding only `first`.
    pub(crate) fn new(first: &str) -> Self {
        Self {
            entries: vec![first.to_owned()],
            index: 0,
        }
    }

    /// The address showing.
    pub(crate) fn current(&self) -> &str {
        &self.entries[self.index]
    }

    /// Visits `address`, dropping any entries forward of the current one.
    pub(crate) fn push(&mut self, address: &str) {
        self.entries.truncate(self.index + 1);
        self.entries.push(address.to_owned());
        self.index += 1;
    }

    /// Moves back one entry. `false` if already at the first.
    pub(crate) fn back(&mut self) -> bool {
        let moved = self.index > 0;
        if moved {
            self.index -= 1;
        }
        moved
    }

    pub(crate) fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub(crate) fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    /// Moves forward one entry. `false` if already at the last.
    pub(crate) fn forward(&mut self) -> bool {
        let moved = self.index + 1 < self.entries.len();
        if moved {
            self.index += 1;
        }
        moved
    }
}

/// Where a link with `href` leads from the page at `current`, or `None` if
/// it leads nowhere the viewer opens: an in-page `#fragment`, another scheme
/// such as `mailto:`, or a relative link from the start page.
pub(crate) fn resolve_link(current: &str, href: &str) -> Option<String> {
    if href.is_empty() || href.starts_with('#') {
        return None;
    }
    if is_remote_url(href) {
        return Some(href.to_owned());
    }
    if has_scheme(href) {
        return None;
    }
    // A remote page links only to remote pages: never to files on this computer.
    if is_remote_url(current) {
        let folder = &current[..current.rfind('/').map_or(0, |i| i + 1)];
        return Some(format!("{folder}{href}"));
    }
    if current.is_empty() {
        return None;
    }

    let current = Path::new(current);
    let target = Path::new(href);
    let stem = target.file_stem()?.to_string_lossy();
    let parent = current.parent().unwrap_or(Path::new(""));
    let resolved = match extension(current).as_str() {
        "vpp" => parent.join(href),
        // Development: a link to `about.vpp` opens the page source `about.html`.
        "html" | "htm" if extension(target) == "vpp" => parent.join(format!("{stem}.html")),
        "html" | "htm" => parent.join(href),
        // A `dist/<page>` folder, or a file in it: the sibling folder `dist/<target>`.
        _ => {
            let folder = if current.is_dir() { current } else { parent };
            folder.parent().unwrap_or(Path::new("")).join(&*stem)
        }
    };
    Some(resolved.to_string_lossy().into_owned())
}

/// What to open for an address typed into the address field. Something
/// that looks like a host and path, such as `example.com/site/home.vpp`,
/// and is not a file here, gets `https://`.
pub(crate) fn typed_address(text: &str) -> String {
    let text = text.trim();
    let looks_like_web = !is_remote_url(text)
        && text.contains('.')
        && text.contains('/')
        && !text.contains('\\')
        && !text.starts_with('.')
        && !Path::new(text).exists();
    if looks_like_web {
        format!("https://{text}")
    } else {
        text.to_owned()
    }
}

/// Whether `href` starts with a URL scheme such as `mailto:`. A single letter
/// before the colon is a Windows drive, not a scheme.
fn has_scheme(href: &str) -> bool {
    let Some((scheme, _)) = href.split_once(':') else {
        return false;
    };
    scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(current: &str, href: &str) -> Option<String> {
        resolve_link(current, href).map(|s| s.replace('\\', "/"))
    }

    #[test]
    fn history_moves_and_forgets_the_future() {
        let mut history = History::new("a");
        history.push("b");
        history.push("c");
        assert!(history.back());
        assert!(history.back());
        assert!(!history.back());
        assert_eq!(history.current(), "a");
        assert!(history.forward());
        history.push("d");
        assert!(!history.forward());
        assert!(history.back());
        assert_eq!(history.current(), "b");
    }

    #[test]
    fn links_between_packages_and_sources() {
        assert_eq!(
            resolve("site/home.vpp", "about.vpp").unwrap(),
            "site/about.vpp"
        );
        assert_eq!(
            resolve("site/pages/home.html", "about.vpp").unwrap(),
            "site/pages/about.html"
        );
        assert_eq!(
            resolve("site/pages/home.html", "other.html").unwrap(),
            "site/pages/other.html"
        );
        assert_eq!(
            resolve("site/dist/home/dom.bin", "about.vpp").unwrap(),
            "site/dist/about"
        );
    }

    #[test]
    fn remote_pages_link_to_remote_pages_only() {
        assert_eq!(
            resolve("https://example.com/site/home.vpp", "about.vpp").unwrap(),
            "https://example.com/site/about.vpp"
        );
        assert_eq!(
            resolve(
                "https://example.com/site/home.vpp",
                "C:/Windows/notepad.vpp"
            )
            .unwrap(),
            "https://example.com/site/C:/Windows/notepad.vpp"
        );
        assert_eq!(
            resolve("site/home.vpp", "vpp://example.com/x.vpp").unwrap(),
            "vpp://example.com/x.vpp"
        );
    }

    #[test]
    fn typed_addresses() {
        assert_eq!(
            typed_address("  example.com/site/home.vpp "),
            "https://example.com/site/home.vpp"
        );
        assert_eq!(
            typed_address("vpp://example.com/a.vpp"),
            "vpp://example.com/a.vpp"
        );
        assert_eq!(typed_address("./site/home.vpp"), "./site/home.vpp");
        assert_eq!(typed_address(r"C:\site\home.vpp"), r"C:\site\home.vpp");
        assert_eq!(typed_address("home.vpp"), "home.vpp");
        let here = format!("{}/src", env!("CARGO_MANIFEST_DIR").replace('\\', "/"));
        assert_eq!(typed_address(&here), here);
    }

    #[test]
    fn links_that_lead_nowhere() {
        assert_eq!(resolve("site/home.vpp", ""), None);
        assert_eq!(resolve("site/home.vpp", "#top"), None);
        assert_eq!(resolve("site/home.vpp", "mailto:a@example.com"), None);
        assert_eq!(resolve("site/home.vpp", "javascript:alert(1)"), None);
        assert_eq!(resolve("", "about.vpp"), None);
    }
}

//! Expanding a whole page: layouts, then includes and components, then
//! collecting the stylesheets and scripts it uses, in load order.

use std::path::{Path, PathBuf};

use vpp_dom::Document;
use vpp_style::StyleSheet;

use crate::error::TemplateError;
use crate::expander::{Expander, MAX_NESTING};
use crate::layout::find_layout_tag;
use crate::project::TemplateOptions;

/// A page with every template element resolved.
#[derive(Debug)]
pub struct ExpandedPage {
    /// The finished document: plain HTML structure, ready for `dom.bin`.
    pub document: Document,
    /// Its stylesheets in load order: `<link>` and `<style>` in document
    /// order, then each component's stylesheet once.
    pub styles: Vec<PageStyle>,
    /// Its scripts in load order.
    pub scripts: Vec<PageScript>,
    /// Things the author should know that did not stop expansion.
    pub warnings: Vec<String>,
}

/// A stylesheet the page uses, already parsed (and scoped, for components).
#[derive(Debug, Clone)]
pub struct PageStyle {
    /// The resource name, such as `global`, `inline-1`, or `component-stat-card`.
    pub name: String,
    /// The source file; `None` for an inline `<style>`.
    pub path: Option<PathBuf>,
    /// The parsed rules.
    pub sheet: StyleSheet,
}

/// A script the page runs.
#[derive(Debug, Clone)]
pub struct PageScript {
    /// The resource name, such as `app` or `inline-1`.
    pub name: String,
    /// The source file; `None` for an inline `<script>`.
    pub path: Option<PathBuf>,
    /// The JavaScript source.
    pub source: String,
}

/// Loads a page, applies its layout, expands includes and components, and
/// collects its stylesheets and scripts. Plain pages without template
/// elements work too.
pub fn expand_page(
    page_file: &Path,
    options: &TemplateOptions,
) -> Result<ExpandedPage, TemplateError> {
    let mut expander = Expander::new(options)?;
    let mut document = expander.load_document(page_file)?;

    for _ in 0..MAX_NESTING {
        let Some(tag) = find_layout_tag(&document) else {
            break;
        };
        document = expander.apply_layout(document, tag)?;
    }
    if find_layout_tag(&document).is_some() {
        return Err(TemplateError::new(page_file, "layouts nest too deeply"));
    }

    let root = document.root();
    expander.expand_children(&mut document, root, 0)?;
    let (styles, scripts) = expander.collect_resources(&document)?;
    Ok(ExpandedPage {
        document,
        styles,
        scripts,
        warnings: expander.warnings,
    })
}

impl Expander<'_> {
    fn collect_resources(
        &mut self,
        document: &Document,
    ) -> Result<(Vec<PageStyle>, Vec<PageScript>), TemplateError> {
        let mut styles = Vec::new();
        let mut scripts = Vec::new();
        let (mut inline_styles, mut inline_scripts) = (0, 0);

        for node in document.descendants(document.root()) {
            let Some(element) = document.element(node) else {
                continue;
            };
            match element.tag.as_str() {
                "link" => {
                    let is_stylesheet = element.attribute("rel").is_some_and(|rel| {
                        rel.split_ascii_whitespace()
                            .any(|r| r.eq_ignore_ascii_case("stylesheet"))
                    });
                    let Some(href) = element.attribute("href").filter(|_| is_stylesheet) else {
                        continue;
                    };
                    let file = self.local_file(href)?;
                    let css = read(&file)?;
                    styles.push(PageStyle {
                        name: sanitize_name(&stem(&file)),
                        path: Some(file),
                        sheet: vpp_style::parse_stylesheet(&css),
                    });
                }
                "style" => {
                    inline_styles += 1;
                    styles.push(PageStyle {
                        name: format!("inline-{inline_styles}"),
                        path: None,
                        sheet: vpp_style::parse_stylesheet(&document.raw_text(node)),
                    });
                }
                "script" => match element.attribute("src") {
                    Some(src) => {
                        let file = self.local_file(src)?;
                        scripts.push(PageScript {
                            name: sanitize_name(&stem(&file)),
                            source: read(&file)?,
                            path: Some(file),
                        });
                    }
                    None => {
                        inline_scripts += 1;
                        scripts.push(PageScript {
                            name: format!("inline-{inline_scripts}"),
                            path: None,
                            source: document.raw_text(node),
                        });
                    }
                },
                _ => {}
            }
        }
        styles.append(&mut self.component_styles);
        Ok((styles, scripts))
    }

    /// A stylesheet or script reference as a project file. Remote ones are not supported.
    fn local_file(&self, reference: &str) -> Result<PathBuf, TemplateError> {
        if reference.contains("://") {
            return Err(TemplateError::new(
                &self.root(),
                format!(
                    "{reference}: remote stylesheets and scripts are not supported; copy the file into the project"
                ),
            ));
        }
        self.resolve_from_root(reference)
    }
}

fn read(file: &Path) -> Result<String, TemplateError> {
    std::fs::read_to_string(file)
        .map_err(|e| TemplateError::new(file, format!("cannot read file: {e}")))
}

fn stem(file: &Path) -> String {
    file.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A resource name from a file name: letters, digits, `-`, and `_` kept, anything else `-`.
pub(crate) fn sanitize_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if clean.is_empty() {
        "resource".to_owned()
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;

    fn options(dir: &TestDir) -> TemplateOptions {
        TemplateOptions {
            project_root: dir.path().to_path_buf(),
            ..TemplateOptions::default()
        }
    }

    fn body_text(page: &ExpandedPage) -> String {
        let body = page
            .document
            .find_first(page.document.root(), "body")
            .unwrap();
        page.document.text_content(body)
    }

    #[test]
    fn plain_page_passes_through_with_its_resources() {
        let dir = TestDir::new("plain");
        dir.write("styles/global.css", b"p { color: red }");
        dir.write("scripts/app.js", b"console.log(1)");
        dir.write(
            "pages/home.html",
            br#"<link rel="stylesheet" href="../styles/global.css"><style>b{x:1}</style>
                <p>hi</p><script src="/scripts/app.js"></script><script>go()</script>"#,
        );
        let page = expand_page(&dir.path().join("pages/home.html"), &options(&dir)).unwrap();
        let p = page.document.find_first(page.document.root(), "p").unwrap();
        assert_eq!(page.document.text_content(p), "hi");
        let styles: Vec<_> = page.styles.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(styles, ["global", "inline-1"]);
        let scripts: Vec<_> = page
            .scripts
            .iter()
            .map(|s| (s.name.as_str(), s.source.as_str()))
            .collect();
        assert_eq!(scripts, [("app", "console.log(1)"), ("inline-1", "go()")]);
    }

    #[test]
    fn layout_include_and_component_together() {
        let dir = TestDir::new("full");
        dir.write(
            "layouts/main.html",
            br#"<head><vpp-slot name="head" /></head><body>[<vpp-slot />]<vpp-include src="/includes/f.html" who="me" /></body>"#,
        );
        dir.write("includes/f.html", b"<p>by {{ who }}</p>");
        dir.write(
            "components/x-card/component.html",
            br#"<div class="card">{{ n }}:<vpp-slot /></div>"#,
        );
        dir.write("components/x-card/component.css", b".card { color: red }");
        dir.write(
            "pages/home.html",
            br#"<vpp-layout src="/layouts/main.html"><vpp-fill slot="head"><title>T</title></vpp-fill>
                <x-card n="1">one</x-card><x-card n="2" /></vpp-layout>"#,
        );
        let page = expand_page(&dir.path().join("pages/home.html"), &options(&dir)).unwrap();
        // No space between the two cards in the source, so none between them here.
        assert_eq!(body_text(&page), "[1:one2:]by me");

        let doc = &page.document;
        let title = doc.find_first(doc.root(), "title").unwrap();
        assert!(doc.closest(title, "head").is_some());

        // The component's elements and its stylesheet share the scope class.
        let card = doc.find_first(doc.root(), "div").unwrap();
        let class = doc
            .element(card)
            .unwrap()
            .attribute("class")
            .unwrap()
            .to_owned();
        let scope = class.split(' ').nth(1).unwrap();
        assert!(scope.starts_with("vpp-s"));
        assert_eq!(page.styles.len(), 1, "one stylesheet for two cards");
        assert_eq!(page.styles[0].name, "component-x-card");
        assert_eq!(
            page.styles[0].sheet.rules[0].selectors[0].compounds[0].classes,
            ["card", scope]
        );
    }

    #[test]
    fn deep_ordinary_markup_is_not_a_nesting_error() {
        // The C++ expander counted element depth as include depth, and failed here.
        let dir = TestDir::new("deep");
        let html = format!("{}x{}", "<div>".repeat(60), "</div>".repeat(60));
        dir.write("page.html", html.as_bytes());
        assert!(expand_page(&dir.path().join("page.html"), &options(&dir)).is_ok());
    }

    #[test]
    fn include_at_the_top_of_an_include_is_expanded() {
        // The C++ expander left this nested include unexpanded.
        let dir = TestDir::new("nested-include");
        dir.write("a.html", br#"<vpp-include src="/b.html" />"#);
        dir.write("b.html", b"<p>b</p>");
        dir.write("page.html", br#"<vpp-include src="/a.html" />"#);
        let page = expand_page(&dir.path().join("page.html"), &options(&dir)).unwrap();
        assert_eq!(body_text(&page), "b");
    }

    #[test]
    fn a_template_that_includes_itself_is_an_error() {
        let dir = TestDir::new("loop");
        dir.write("page.html", br#"<p><vpp-include src="/page.html" /></p>"#);
        let err = expand_page(&dir.path().join("page.html"), &options(&dir)).unwrap_err();
        assert!(err.message.contains("nest too deeply"));
    }

    #[test]
    fn missing_files_and_escapes_are_reported() {
        let dir = TestDir::new("errors");
        dir.write("a.html", br#"<vpp-include src="/missing.html" />"#);
        dir.write(
            "b.html",
            br#"<vpp-include src="/missing.html" optional />ok"#,
        );
        dir.write("c.html", br#"<img src="../../outside.png">"#);
        dir.write(
            "d.html",
            br#"<img src="data:image/png;base64,AAAA"><a href="../../x.vpp">x</a>"#,
        );
        let run = |f: &str| expand_page(&dir.path().join(f), &options(&dir));

        assert!(
            run("a.html")
                .unwrap_err()
                .message
                .contains("include not found: /missing.html")
        );
        assert_eq!(body_text(&run("b.html").unwrap()), "ok");
        assert!(
            run("c.html")
                .unwrap_err()
                .message
                .contains("leaves the project")
        );
        // data: URLs and links between pages are left as written.
        let page = run("d.html").unwrap();
        let img = page
            .document
            .find_first(page.document.root(), "img")
            .unwrap();
        assert!(
            page.document
                .element(img)
                .unwrap()
                .attribute("src")
                .unwrap()
                .starts_with("data:")
        );
    }

    #[test]
    fn component_javascript_is_a_warning() {
        let dir = TestDir::new("component-js");
        dir.write("components/x-a/component.html", b"<i>a</i>");
        dir.write("components/x-a/component.js", b"");
        dir.write("page.html", b"<x-a></x-a>");
        let page = expand_page(&dir.path().join("page.html"), &options(&dir)).unwrap();
        assert_eq!(page.warnings.len(), 1);
        assert!(page.warnings[0].contains("component.js ignored"));
    }

    #[test]
    fn sanitizes_resource_names() {
        assert_eq!(sanitize_name("my style.v2"), "my-style-v2");
        assert_eq!(sanitize_name(""), "resource");
    }
}

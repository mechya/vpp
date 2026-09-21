//! Compiling pages: each page is expanded with its layout, includes, and
//! components, then written as
//!
//! ```text
//! dist/<page>/dom.bin                 the finished page
//! dist/<page>/style/NN-<name>.bin     one per stylesheet, in load order
//! dist/<page>/code/NN-<name>.bin      one per script, in load order
//! ```
//!
//! One resource per source file lets pages that share a stylesheet or script
//! share the resource by hash.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vpp_format::{CodeBin, SiteConfig};
use vpp_template::{PageScript, TemplateOptions, expand_page, find_project_root};

/// Compiles every page of the site at `root` into `dist_root`: each
/// `pages/*.html`, or a lone `index.html` as the page `index`.
pub(crate) fn compile_project(root: &Path, dist_root: &Path, out: &mut dyn Write) -> Result<()> {
    let pages = find_pages(root)?;
    let plural = if pages.len() == 1 { "" } else { "s" };
    writeln!(
        out,
        "vppc: {} ({} page{plural})",
        root.display(),
        pages.len()
    )?;
    let options = template_options(root)?;
    for page in &pages {
        compile_page(root, page, dist_root, &options, out)?;
    }
    writeln!(out, "vppc: written to {}", dist_root.display())?;
    Ok(())
}

/// Compiles one page into its project's `dist/`.
pub(crate) fn compile_single_page(page: &Path, out: &mut dyn Write) -> Result<()> {
    let page = std::path::absolute(page).with_context(|| page.display().to_string())?;
    let root = find_project_root(&page);
    writeln!(out, "vppc: {} (project {})", page.display(), root.display())?;
    let dist_root = root.join("dist");
    compile_page(&root, &page, &dist_root, &template_options(&root)?, out)?;
    writeln!(
        out,
        "vppc: written to {}",
        dist_root.join(page_name(&page)).display()
    )?;
    Ok(())
}

/// Checks one script and writes it as a `code.bin` container, to `output` or
/// to `code.bin` next to the script.
pub(crate) fn compile_script(
    file: &Path,
    output: Option<&Path>,
    out: &mut dyn Write,
) -> Result<()> {
    let source =
        fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
    let bytes = code_bin(&file_name(file), &source)?;
    let output = match output {
        Some(path) => path.to_path_buf(),
        None => file.with_file_name("code.bin"),
    };
    write_file(&output, &bytes)?;
    writeln!(
        out,
        "vppc: {} -> {} ({} bytes)",
        file.display(),
        output.display(),
        bytes.len()
    )?;
    Ok(())
}

/// Checks a script's syntax, then wraps its source in a `code.bin` container
/// (`docs/design/0001-code-bin.md`). Nothing in the script runs.
fn code_bin(filename: &str, source: &str) -> Result<Vec<u8>> {
    vpp_script::check_syntax(source, filename)?;
    Ok(CodeBin::classic(filename, source).encode())
}

/// The name a script's errors are reported under: its file name, or `<name>.js` for inline scripts.
fn script_file_name(script: &PageScript) -> String {
    match &script.path {
        Some(path) => file_name(path),
        None => format!("{}.js", script.name),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn compile_page(
    root: &Path,
    page_file: &Path,
    dist_root: &Path,
    options: &TemplateOptions,
    out: &mut dyn Write,
) -> Result<()> {
    let page = expand_page(page_file, options)?;
    for warning in &page.warnings {
        writeln!(out, "  warning  {warning}")?;
    }

    // Every script is checked before anything is written, so a syntax error
    // leaves the previous output in place.
    let mut scripts = Vec::with_capacity(page.scripts.len());
    for (i, script) in page.scripts.iter().enumerate() {
        let name = format!("code/{i:02}-{}.bin", script.name);
        scripts.push((name, code_bin(&script_file_name(script), &script.source)?));
    }

    let dist = dist_root.join(page_name(page_file));
    if dist.exists() {
        fs::remove_dir_all(&dist).with_context(|| format!("cannot clear {}", dist.display()))?;
    }

    let dom = vpp_dom::encode_dom(&page.document);
    write_file(&dist.join("dom.bin"), &dom)?;
    let shown = page_file.strip_prefix(root).unwrap_or(page_file);
    writeln!(
        out,
        "  page     {:<28} dom.bin {} bytes",
        shown.to_string_lossy().replace('\\', "/"),
        dom.len()
    )?;

    for (i, style) in page.styles.iter().enumerate() {
        let name = format!("style/{i:02}-{}.bin", style.name);
        let bytes = vpp_style::encode_stylesheet(&style.sheet);
        write_file(&dist.join(&name), &bytes)?;
        writeln!(
            out,
            "    {name:<30} {} rules, {} bytes",
            style.sheet.rules.len(),
            bytes.len()
        )?;
    }

    for (name, bytes) in scripts {
        write_file(&dist.join(&name), &bytes)?;
        writeln!(out, "    {name:<30} {} bytes", bytes.len())?;
    }
    Ok(())
}

/// `pages/*.html` (or `.htm`), sorted; or a lone `index.html`.
fn find_pages(root: &Path) -> Result<Vec<PathBuf>> {
    let pages_dir = root.join("pages");
    let mut pages = Vec::new();
    if pages_dir.is_dir() {
        for entry in fs::read_dir(&pages_dir).with_context(|| pages_dir.display().to_string())? {
            let path = entry?.path();
            let is_html = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
            if path.is_file() && is_html {
                pages.push(path);
            }
        }
        pages.sort();
    } else if root.join("index.html").is_file() {
        pages.push(root.join("index.html"));
    }
    if pages.is_empty() {
        bail!("{} has no pages/*.html and no index.html", root.display());
    }
    Ok(pages)
}

/// The template options for a project: its root and the component aliases in `vpp.json`, if any.
fn template_options(root: &Path) -> Result<TemplateOptions> {
    let config_path = root.join("vpp.json");
    let component_aliases = match fs::read_to_string(&config_path) {
        Ok(text) => SiteConfig::parse(&text)
            .with_context(|| config_path.display().to_string())?
            .components
            .into_iter()
            .map(|(tag, dir)| (tag.to_ascii_lowercase(), dir))
            .collect(),
        Err(_) => Default::default(),
    };
    Ok(TemplateOptions {
        project_root: root.to_path_buf(),
        component_aliases,
    })
}

fn page_name(page_file: &Path) -> String {
    page_file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "index".to_owned())
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

    #[test]
    fn compiles_every_page_with_its_styles() {
        let dir = TestDir::new("project");
        dir.write(
            "vpp.json",
            br#"{ "id": "t", "components": { "X-Card": "/components/card" } }"#,
        );
        dir.write("components/card/component.html", b"<b>card</b>");
        dir.write("styles/global.css", b"p { color: red }");
        dir.write(
            "pages/home.html",
            br#"<link rel="stylesheet" href="/styles/global.css"><p>home</p><x-card />"#,
        );
        dir.write("pages/about.htm", b"<p>about</p><script>go()</script>");
        dir.write("pages/notes.txt", b"not a page");
        let dist = dir.path().join("dist");
        let mut log = Vec::new();
        compile_project(dir.path(), &dist, &mut log).unwrap();
        let log = String::from_utf8(log).unwrap();
        assert!(log.contains("(2 pages)"), "{log}");

        let home = vpp_dom::decode_dom(&fs::read(dist.join("home/dom.bin")).unwrap()).unwrap();
        let body = home.find_first(home.root(), "body").unwrap();
        // The alias is matched case-insensitively, like HTML tag names.
        assert_eq!(home.text_content(body), "homecard");
        let style = fs::read(dist.join("home/style/00-global.bin")).unwrap();
        assert_eq!(vpp_style::decode_stylesheet(&style).unwrap().rules.len(), 1);
        assert!(dist.join("about/dom.bin").is_file());

        let code =
            CodeBin::decode(&fs::read(dist.join("about/code/00-inline-1.bin")).unwrap()).unwrap();
        assert_eq!(code.filename, "inline-1.js");
        assert_eq!(code.source, "go()");
    }

    #[test]
    fn a_syntax_error_stops_the_page_and_keeps_the_old_output() {
        let dir = TestDir::new("syntax");
        dir.write("index.html", br#"<p>x</p><script src="/app.js"></script>"#);
        dir.write("app.js", b"let ok = 1;\nlet broken = (;\n");
        dir.write("dist/index/dom.bin", b"previous build");
        let err =
            compile_project(dir.path(), &dir.path().join("dist"), &mut Vec::new()).unwrap_err();
        assert!(err.to_string().starts_with("app.js:2:"), "{err}");
        assert_eq!(
            fs::read(dir.path().join("dist/index/dom.bin")).unwrap(),
            b"previous build"
        );
    }

    #[test]
    fn a_single_script_becomes_code_bin() {
        let dir = TestDir::new("script");
        dir.write("app.js", b"console.log(1)");
        let mut log = Vec::new();
        compile_script(&dir.path().join("app.js"), None, &mut log).unwrap();
        let code = CodeBin::decode(&fs::read(dir.path().join("code.bin")).unwrap()).unwrap();
        assert_eq!(code.filename, "app.js");
        assert_eq!(code.source, "console.log(1)");

        let chosen = dir.path().join("out/app.bin");
        compile_script(&dir.path().join("app.js"), Some(&chosen), &mut log).unwrap();
        assert!(chosen.is_file());
    }

    #[test]
    fn recompiling_clears_old_output() {
        let dir = TestDir::new("clear");
        dir.write("index.html", b"<p>one</p>");
        dir.write("dist/index/style/00-stale.bin", b"old");
        compile_project(dir.path(), &dir.path().join("dist"), &mut Vec::new()).unwrap();
        assert!(dir.path().join("dist/index/dom.bin").is_file());
        assert!(!dir.path().join("dist/index/style").exists());
    }

    #[test]
    fn explains_a_project_without_pages() {
        let dir = TestDir::new("empty");
        let err =
            compile_project(dir.path(), &dir.path().join("dist"), &mut Vec::new()).unwrap_err();
        assert!(
            err.to_string()
                .contains("no pages/*.html and no index.html")
        );
    }

    #[test]
    fn template_errors_name_the_file() {
        let dir = TestDir::new("bad");
        dir.write("index.html", br#"<vpp-include src="/missing.html" />"#);
        let err =
            compile_project(dir.path(), &dir.path().join("dist"), &mut Vec::new()).unwrap_err();
        assert!(
            err.to_string().contains("include not found: /missing.html"),
            "{err}"
        );
    }
}

//! Screenshot tests: every example page, expanded, laid out, and painted with
//! the test fonts (DejaVu Sans), compared with a reference image approved by
//! a person (`docs/rust-port.md` §8, step 5).
//!
//! When a rendering change is intended, regenerate the references with
//! `VPP_UPDATE_SCREENSHOTS=1 cargo test -p vpp-paint --test screenshots`,
//! look at every changed image, and commit them with the change.

use std::fs;
use std::path::{Path, PathBuf};

use vpp_paint::{Font, FontSet, Pixmap, render_page};
use vpp_template::{TemplateOptions, expand_page};

/// How far a channel may differ, and how many pixels may differ at all, before
/// a screenshot fails. Small anti-aliasing differences between CPUs pass.
const CHANNEL_TOLERANCE: u8 = 3;
const PIXEL_TOLERANCE: f64 = 0.001;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fonts() -> FontSet {
    let dir = repo().join("tests/fixtures/fonts");
    FontSet {
        regular: Font::load(&dir.join("DejaVuSans.ttf")).unwrap(),
        bold: Some(Font::load(&dir.join("DejaVuSans-Bold.ttf")).unwrap()),
    }
}

fn render(site: &str, page: &str) -> Pixmap {
    let root = repo().join("examples").join(site);
    let config = fs::read_to_string(root.join("vpp.json")).unwrap();
    let options = TemplateOptions {
        project_root: root.clone(),
        component_aliases: vpp_format::SiteConfig::parse(&config).unwrap().components,
    };
    let expanded = expand_page(&root.join("pages").join(format!("{page}.html")), &options).unwrap();
    let sheets: Vec<_> = expanded.styles.into_iter().map(|s| s.sheet).collect();
    let fonts = fonts();
    let layout = vpp_layout::layout_document(&expanded.document, &sheets, &fonts, 800.0).unwrap();
    render_page(&layout, &expanded.document, &fonts, 1.0)
}

fn check(site: &str, page: &str) {
    let name = format!("{site}-{page}.png");
    let reference = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/screenshots")
        .join(&name);
    let actual = render(site, page);

    // INTENTIONAL: a test-only switch that rewrites the reference images. It is
    // off unless set, and every image it writes must be looked at before committing.
    if std::env::var_os("VPP_UPDATE_SCREENSHOTS").is_some() {
        fs::create_dir_all(reference.parent().unwrap()).unwrap();
        actual.save_png(&reference).unwrap();
        return;
    }

    let Ok(expected) = Pixmap::load_png(&reference) else {
        panic!(
            "no reference image {}; create it with VPP_UPDATE_SCREENSHOTS=1 and look at it",
            reference.display()
        );
    };
    let differences = difference(&expected, &actual);
    if let Some(problem) = differences {
        let out = repo().join("target/screenshots");
        fs::create_dir_all(&out).unwrap();
        let saved = out.join(name.replace(".png", "-actual.png"));
        actual.save_png(&saved).unwrap();
        panic!(
            "{site}/{page}: {problem}; the new rendering is in {}",
            saved.display()
        );
    }
}

/// Why two images differ too much, if they do.
fn difference(expected: &Pixmap, actual: &Pixmap) -> Option<String> {
    if (expected.width(), expected.height()) != (actual.width(), actual.height()) {
        return Some(format!(
            "size changed from {}x{} to {}x{}",
            expected.width(),
            expected.height(),
            actual.width(),
            actual.height()
        ));
    }
    let differing = expected
        .data()
        .chunks_exact(4)
        .zip(actual.data().chunks_exact(4))
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let total = (expected.width() * expected.height()) as f64;
    (differing as f64 / total > PIXEL_TOLERANCE).then(|| format!("{differing} pixels differ"))
}

#[test]
fn hello_world_home() {
    check("hello-world", "home");
}

#[test]
fn hello_world_about() {
    check("hello-world", "about");
}

#[test]
fn app_window_home() {
    check("app-window", "home");
}

#[test]
fn app_window_about() {
    check("app-window", "about");
}

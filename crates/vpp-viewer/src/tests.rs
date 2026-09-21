//! The viewer without a window: the example sites in every mode the viewer
//! opens, clicked through as a user would (`docs/reference/viewer.md`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use vpp_dom::{Document, NodeId, encode_dom};
use vpp_format::{Manifest, Package, SigningKey, SiteConfig};
use vpp_style::encode_stylesheet;
use vpp_template::{TemplateOptions, expand_page};
use vpp_updater::{Fetch, FetchError};

use crate::{
    Font, FontSet, Key, Modifiers, ResizeEdge, Viewer, ViewerOptions, WindowCommand, WindowRegion,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn page_source(site: &str, page: &str) -> String {
    let path = repo()
        .join("examples")
        .join(site)
        .join("pages")
        .join(format!("{page}.html"));
    path.to_string_lossy().into_owned()
}

/// A fresh temporary folder, removed when dropped.
struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vpp-viewer-test-{}-{n}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A web server held in memory.
struct Server(BTreeMap<String, Vec<u8>>);

impl Fetch for Server {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.0.get(url).cloned().ok_or_else(|| FetchError::Status {
            url: url.to_owned(),
            status: 404,
        })
    }
}

struct Setup {
    data: TestDir,
    allow_unsigned: bool,
    server: BTreeMap<String, Vec<u8>>,
}

impl Setup {
    fn new() -> Self {
        Self {
            data: TestDir::new("data"),
            allow_unsigned: false,
            server: BTreeMap::new(),
        }
    }

    fn open(&self, address: &str) -> Viewer {
        let fonts_dir = repo().join("tests/fixtures/fonts");
        let options = ViewerOptions {
            fonts: FontSet {
                regular: Font::load(&fonts_dir.join("DejaVuSans.ttf")).unwrap(),
                bold: Some(Font::load(&fonts_dir.join("DejaVuSans-Bold.ttf")).unwrap()),
            },
            data_dir: self.data.0.clone(),
            allow_unsigned: self.allow_unsigned,
            fetch: Box::new(Server(self.server.clone())),
        };
        Viewer::new(options, address, WIDTH, HEIGHT, 1.0)
    }
}

/// Clicks the middle of the element `pick` finds.
fn click(viewer: &mut Viewer, pick: impl FnOnce(&Document) -> Option<NodeId>) {
    let rect = viewer
        .page
        .element_box(pick)
        .expect("the element has a box");
    let scale = viewer.scale;
    let (x, y) = (
        (rect.x + rect.w / 2.0) * scale,
        (rect.y + rect.h / 2.0) * scale,
    );
    viewer.pointer_down(x, y);
    viewer.pointer_up(x, y);
}

/// Clicks the middle of the shell control with `id`.
fn click_shell(viewer: &mut Viewer, id: &str) {
    let rect = viewer.shell.box_of(id).expect("the control is shown");
    let scale = viewer.scale;
    let (x, y) = (
        (rect.x + rect.w / 2.0) * scale,
        (rect.y + rect.h / 2.0) * scale,
    );
    viewer.pointer_down(x, y);
    viewer.pointer_up(x, y);
}

fn click_id(viewer: &mut Viewer, id: &str) {
    click(viewer, |d| d.find_by_id(d.root(), id));
}

fn click_link(viewer: &mut Viewer, href: &str) {
    click(viewer, |d| {
        d.descendants(d.root())
            .find(|&n| d.element(n).and_then(|e| e.attribute("href")) == Some(href))
    });
}

fn press(viewer: &mut Viewer, key: Key) {
    viewer.key_down(key, Modifiers::default());
}

fn press_ctrl(viewer: &mut Viewer, letter: &str) {
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    viewer.key_down(Key::Character(letter.into()), ctrl);
}

/// The window commands queued that scripts and keys cause.
fn window_commands(viewer: &mut Viewer) -> Vec<WindowCommand> {
    let mut commands = viewer.take_commands();
    commands.retain(|c| {
        matches!(
            c,
            WindowCommand::Close | WindowCommand::Minimize | WindowCommand::ToggleMaximize
        )
    });
    commands
}

/// The hello-world home page, compiled as `vppc` does.
fn compiled_home() -> (Manifest, BTreeMap<String, Vec<u8>>) {
    let root = repo().join("examples/hello-world");
    let config = SiteConfig::parse(&fs::read_to_string(root.join("vpp.json")).unwrap()).unwrap();
    let options = TemplateOptions {
        project_root: root.clone(),
        component_aliases: config.components.clone(),
    };
    let expanded = expand_page(&root.join("pages/home.html"), &options).unwrap();
    let mut resources = BTreeMap::new();
    resources.insert("dom.bin".to_owned(), encode_dom(&expanded.document));
    for (i, style) in expanded.styles.iter().enumerate() {
        let name = format!("style/{i:02}-{}.bin", style.name);
        resources.insert(name, encode_stylesheet(&style.sheet));
    }
    for (i, script) in expanded.scripts.iter().enumerate() {
        let code = vpp_format::CodeBin::classic(format!("{}.js", script.name), &script.source);
        resources.insert(format!("code/{i:02}-{}.bin", script.name), code.encode());
    }
    (config.manifest("home"), resources)
}

fn write_package(dir: &TestDir, key: Option<&SigningKey>) -> String {
    let (manifest, resources) = compiled_home();
    let file = dir.0.join("home.vpp");
    fs::write(&file, Package::build(&manifest, &resources, key)).unwrap();
    file.to_string_lossy().into_owned()
}

/// The hello-world counter works: the page is running, laid out, and clickable.
fn assert_counter_works(viewer: &mut Viewer) {
    click_id(viewer, "hello");
    assert_eq!(viewer.page.text_of("count-value").unwrap(), "1");
}

#[test]
fn the_start_page_shows_without_an_address() {
    let mut viewer = Setup::new().open("");
    assert_eq!(viewer.page.title, "VPP Viewer");
    assert_eq!(
        viewer.take_commands()[0],
        WindowCommand::SetTitle("VPP Viewer".into())
    );
    assert!(viewer.take_log().contains(&"mode: start".to_owned()));
}

#[test]
fn development_page_runs_its_scripts_and_opens_a_popup() {
    let mut viewer = Setup::new().open(&page_source("hello-world", "home"));
    assert_eq!(viewer.page.title, "Home");

    click_id(&mut viewer, "hello");
    assert_eq!(viewer.page.text_of("hello").unwrap(), "Clicked 1");
    assert!(
        viewer
            .take_log()
            .contains(&"console: clicked hello count 1".to_owned())
    );

    // The popup's white card covers the middle of the window.
    let middle = viewer.render().pixel(WIDTH / 2, HEIGHT / 2).unwrap();
    assert_eq!(
        (middle.red(), middle.green(), middle.blue()),
        (255, 255, 255)
    );

    // While it is open, the page gets no clicks.
    click_id(&mut viewer, "hello");
    assert_eq!(viewer.page.text_of("hello").unwrap(), "Clicked 1");

    // Esc closes the popup, and then the window.
    press(&mut viewer, Key::Escape);
    assert_eq!(window_commands(&mut viewer), []);
    click_id(&mut viewer, "hello");
    assert_eq!(viewer.page.text_of("hello").unwrap(), "Clicked 2");
    press(&mut viewer, Key::Escape);
    press(&mut viewer, Key::Escape);
    assert_eq!(window_commands(&mut viewer), [WindowCommand::Close]);
}

#[test]
fn links_navigate_and_history_goes_back_and_forward() {
    let mut viewer = Setup::new().open(&page_source("hello-world", "home"));
    click_link(&mut viewer, "about.vpp");
    assert_eq!(viewer.page.title, "About");
    assert!(
        viewer
            .address()
            .replace('\\', "/")
            .ends_with("pages/about.html")
    );

    viewer.back();
    assert_eq!(viewer.page.title, "Home");
    viewer.forward();
    assert_eq!(viewer.page.title, "About");
    viewer.forward();
    assert_eq!(viewer.page.title, "About");
}

#[test]
fn page_scripts_drive_the_window() {
    let mut viewer = Setup::new().open(&page_source("app-window", "home"));
    for id in ["minimize", "maximize", "close"] {
        click_id(&mut viewer, id);
    }
    assert_eq!(
        window_commands(&mut viewer),
        [
            WindowCommand::Minimize,
            WindowCommand::ToggleMaximize,
            WindowCommand::Close
        ]
    );
}

#[test]
fn a_signed_package_runs_and_its_publisher_is_pinned() {
    let setup = Setup::new();
    let site = TestDir::new("site");
    let package = write_package(&site, Some(&SigningKey::from_seed([1; 32])));
    let mut viewer = setup.open(&package);
    assert_eq!(viewer.page.title, "Home");
    assert!(
        viewer
            .take_log()
            .contains(&"publisher: trusted on first use".to_owned())
    );
    assert_counter_works(&mut viewer);

    // The same site signed by someone else is refused.
    let package = write_package(&site, Some(&SigningKey::from_seed([2; 32])));
    let viewer = setup.open(&package);
    assert_eq!(viewer.page.title, "Package rejected");
    assert!(
        viewer
            .page
            .text_of("detail")
            .unwrap()
            .contains("publisher key changed")
    );
}

#[test]
fn a_changed_package_is_refused() {
    let site = TestDir::new("site");
    let package = write_package(&site, Some(&SigningKey::from_seed([1; 32])));
    let mut bytes = fs::read(&package).unwrap();
    // INTENTIONAL: hostile input. One changed byte in the last resource must reject the package.
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(&package, bytes).unwrap();

    let viewer = Setup::new().open(&package);
    assert_eq!(viewer.page.title, "Package rejected");
}

#[test]
fn unsigned_packages_need_allow_unsigned() {
    let site = TestDir::new("site");
    let package = write_package(&site, None);
    let mut setup = Setup::new();
    assert_eq!(setup.open(&package).page.title, "Package rejected");

    setup.allow_unsigned = true;
    let mut viewer = setup.open(&package);
    assert_eq!(viewer.page.title, "Home");
    assert_counter_works(&mut viewer);
}

#[test]
fn a_dist_folder_runs() {
    let dist = TestDir::new("dist");
    let (_, resources) = compiled_home();
    for (name, bytes) in &resources {
        let file = dist.0.join(name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, bytes).unwrap();
    }
    let mut viewer = Setup::new().open(&dist.0.to_string_lossy());
    assert_eq!(viewer.page.title, "Home");
    assert_counter_works(&mut viewer);
}

#[test]
fn a_remote_page_is_installed_and_runs() {
    let site = TestDir::new("site");
    let package = write_package(&site, Some(&SigningKey::from_seed([1; 32])));
    let mut setup = Setup::new();
    setup.server.insert(
        "https://example.com/hello/home.vpp".into(),
        fs::read(package).unwrap(),
    );
    let mut viewer = setup.open("https://example.com/hello/home.vpp");
    assert_eq!(viewer.page.title, "Home");
    assert!(
        viewer
            .take_log()
            .iter()
            .any(|line| line.starts_with("update: "))
    );
    assert_counter_works(&mut viewer);

    let viewer = setup.open("https://example.com/hello/missing.vpp");
    assert_eq!(viewer.page.title, "Could not fetch page");
}

#[test]
fn error_details_are_shown_as_text() {
    let viewer = Setup::new().open("missing-<b>bold</b>.vpp");
    assert_eq!(viewer.page.title, "Could not open package");
    assert!(
        viewer
            .page
            .text_of("detail")
            .unwrap()
            .contains("<b>bold</b>")
    );
}

#[test]
fn resizing_redraws_at_the_new_size() {
    let mut viewer = Setup::new().open(&page_source("hello-world", "home"));
    viewer.resize(400, 300, 2.0);
    assert!(viewer.needs_redraw());
    let frame = viewer.render();
    assert_eq!((frame.width(), frame.height()), (400, 300));
    assert!(!viewer.needs_redraw());
    // Still clickable at the new scale.
    assert_counter_works(&mut viewer);
}

#[test]
fn the_shell_sits_above_the_page() {
    let mut viewer = Setup::new().open(&page_source("hello-world", "home"));
    let bar = viewer.shell.height();
    assert!(bar > 20.0, "{bar}");
    // A long address is shortened, so the window buttons stay in the window.
    assert!(viewer.shell.address.len() > 60);
    let close = viewer.shell.box_of("close").unwrap();
    assert!(close.x + close.w <= WIDTH as f32);
    let hello = viewer
        .page
        .element_box(|d| d.find_by_id(d.root(), "hello"))
        .unwrap();
    assert!(hello.y > bar);

    viewer.take_log();
    click_shell(&mut viewer, "reload");
    let log = viewer.take_log();
    assert!(
        log.iter().any(|line| line.starts_with("mode: development")),
        "{log:?}"
    );
    click_shell(&mut viewer, "maximize");
    assert_eq!(
        window_commands(&mut viewer),
        [WindowCommand::ToggleMaximize]
    );
}

#[test]
fn typing_an_address_opens_it() {
    let mut viewer = Setup::new().open(&page_source("hello-world", "home"));
    click_shell(&mut viewer, "address");
    assert!(viewer.is_editing_address());

    // Esc puts the address back and gives the keyboard back.
    viewer.text_input("nonsense");
    press(&mut viewer, Key::Escape);
    assert!(!viewer.is_editing_address());
    assert_eq!(viewer.shell.address, viewer.address());

    // The text is selected when editing starts, so typing replaces it.
    press_ctrl(&mut viewer, "l");
    viewer.text_input(&page_source("hello-world", "about"));
    viewer.text_input("x");
    press(&mut viewer, Key::Backspace);
    press(&mut viewer, Key::Enter);
    assert_eq!(viewer.page.title, "About");
    assert!(!viewer.is_editing_address());
    click_shell(&mut viewer, "back");
    assert_eq!(viewer.page.title, "Home");
}

#[test]
fn a_site_without_a_title_bar_drags_by_its_top_and_ctrl_l_reveals_the_bar() {
    let mut viewer = Setup::new().open(&page_source("app-window", "home"));
    assert_eq!(viewer.shell.height(), 0.0);
    // The top strip drags the window, except over its buttons.
    assert_eq!(viewer.region_at(200.0, 10.0), WindowRegion::Drag);
    let close = viewer
        .page
        .element_box(|d| d.find_by_id(d.root(), "close"))
        .unwrap();
    assert_eq!(
        viewer.region_at(close.x + close.w / 2.0, close.y + close.h / 2.0),
        WindowRegion::Content
    );

    press_ctrl(&mut viewer, "l");
    assert!(viewer.shell.height() > 0.0);
    assert!(viewer.is_editing_address());
    assert!(viewer.shell.box_of("address").is_some());
    press(&mut viewer, Key::Escape);
    assert_eq!(viewer.shell.height(), 0.0);
}

#[test]
fn window_options_become_window_commands() {
    let mut viewer = Setup::new().open(&page_source("app-window", "home"));
    let commands = viewer.take_commands();
    for expected in [
        WindowCommand::SetResizable(false),
        WindowCommand::SetRoundedCorners(true),
        WindowCommand::SetSize {
            width: Some(800.0),
            height: None,
        },
    ] {
        assert!(commands.contains(&expected), "{expected:?} in {commands:?}");
    }
    // "height": "auto" asks for the height of the page.
    assert!(commands.iter().any(|c| matches!(
        c,
        WindowCommand::SetSize { width: None, height: Some(h) } if *h > 80.0
    )));
}

#[test]
fn the_edges_resize_and_the_bar_drags() {
    let viewer = Setup::new().open(&page_source("hello-world", "home"));
    let (w, h) = (WIDTH as f32, HEIGHT as f32);
    assert_eq!(
        viewer.region_at(2.0, 300.0),
        WindowRegion::Resize(ResizeEdge::West)
    );
    assert_eq!(
        viewer.region_at(w - 2.0, h - 2.0),
        WindowRegion::Resize(ResizeEdge::SouthEast)
    );
    let bar = viewer.shell.height();
    assert_eq!(viewer.region_at(7.0, bar / 2.0), WindowRegion::Drag);
    let back = viewer.shell.box_of("back").unwrap();
    assert_eq!(
        viewer.region_at(back.x + back.w / 2.0, back.y + back.h / 2.0),
        WindowRegion::Content
    );
}

#[test]
fn a_link_into_another_site_shows_the_full_shell() {
    let site = TestDir::new("site-a");
    fs::write(site.0.join("vpp.json"), r#"{"id": "org.example.a"}"#).unwrap();
    let target = page_source("app-window", "home");
    fs::write(
        site.0.join("start.html"),
        format!(r#"<html><body><a href="{target}" style="display: block">App</a></body></html>"#),
    )
    .unwrap();
    let setup = Setup::new();
    let mut viewer = setup.open(&site.0.join("start.html").to_string_lossy());
    click_link(&mut viewer, &target);
    assert_eq!(viewer.page.title, "App Window");
    assert!(
        viewer.shell.height() > 0.0,
        "the landing site cannot hide where the user is"
    );
    press(&mut viewer, Key::Escape);
    assert_eq!(viewer.shell.height(), 0.0);

    // Opened directly, the site starts the way it asked.
    let viewer = setup.open(&target);
    assert_eq!(viewer.shell.height(), 0.0);
}

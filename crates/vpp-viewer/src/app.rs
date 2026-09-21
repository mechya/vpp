//! [`Viewer`]: one window's worth of viewer, with no window.
//!
//! The entry point owns the real window. It tells the viewer about input
//! and size changes, shows the pixels [`Viewer::render`] returns, and carries
//! out the [`WindowCommand`]s the viewer queues. Everything else happens here,
//! so the viewer runs the same on every platform and in tests. Keyboard input
//! and the window's drag and resize areas are in `input.rs`.

use std::path::PathBuf;

use vpp_paint::{FontSet, Pixmap};
use vpp_script::HostRequest;
use vpp_updater::{Fetch, Store, Trust};

use crate::location::Location;
use crate::navigation::{History, resolve_link};
use crate::page::Page;
use crate::page_loader::Loader;
use crate::popup::Popup;
use crate::render::{DEFAULT_BACKGROUND, draw_frame};
use crate::shell::{Control, Shell};
use crate::window_prefs::WindowPrefs;

/// The smallest window height `"height": "auto"` may ask for, in CSS pixels.
const MIN_AUTO_HEIGHT: f32 = 80.0;

/// What the viewer needs from the platform.
pub struct ViewerOptions {
    /// The fonts pages are drawn with.
    pub fonts: FontSet,
    /// The viewer's data folder: pinned publisher keys go in `trust/`,
    /// installed sites in `apps/`.
    pub data_dir: PathBuf,
    /// `--allow-unsigned`: accept unsigned packages, for development only.
    pub allow_unsigned: bool,
    /// How remote pages are downloaded.
    pub fetch: Box<dyn Fetch>,
}

/// Something only the window can do, asked for by the viewer.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowCommand {
    /// Close the window and quit.
    Close,
    /// Minimize the window.
    Minimize,
    /// Maximize the window, or restore it if it is maximized.
    ToggleMaximize,
    /// Show this title.
    SetTitle(String),
    /// Let the user resize the window, or not.
    SetResizable(bool),
    /// Resize the window's contents to this size in CSS pixels; `None` keeps that side.
    SetSize {
        /// The new width.
        width: Option<f32>,
        /// The new height.
        height: Option<f32>,
    },
    /// Round the window's corners, or not.
    SetRoundedCorners(bool),
}

/// The viewer for one window.
pub struct Viewer {
    fonts: FontSet,
    trust: Trust,
    store: Store,
    fetch: Box<dyn Fetch>,
    allow_unsigned: bool,
    pub(crate) history: History,
    pub(crate) page: Page,
    pub(crate) shell: Shell,
    pub(crate) popup: Option<Popup>,
    /// The site the shell was last shown for, to notice links into another site.
    shown_site_id: String,
    /// Whether the page loading was reached by following a link.
    following_link: bool,
    canvas: Pixmap,
    /// Device pixels per CSS pixel.
    pub(crate) scale: f32,
    pub(crate) maximized: bool,
    /// The height last asked for by `"height": "auto"`, so it is asked once.
    requested_height: Option<f32>,
    /// Whether the canvas is out of date.
    dirty: bool,
    pub(crate) log: Vec<String>,
    pub(crate) commands: Vec<WindowCommand>,
}

impl Viewer {
    /// A viewer showing `address` (empty for the start page) in a window
    /// `width` × `height` device pixels, with `scale` device pixels per CSS pixel.
    pub fn new(options: ViewerOptions, address: &str, width: u32, height: u32, scale: f32) -> Self {
        let trust = Trust::new(options.data_dir.join("trust"));
        let store = Store::new(options.data_dir.join("apps"));
        let mut log = Vec::new();
        let loader = Loader {
            trust: &trust,
            store: &store,
            fetch: &*options.fetch,
            allow_unsigned: options.allow_unsigned,
        };
        let loaded = loader.load(&Location::parse(address), &mut log);
        let page = Page::start(loaded, &mut log);
        let mut viewer = Self {
            fonts: options.fonts,
            trust,
            store,
            fetch: options.fetch,
            allow_unsigned: options.allow_unsigned,
            history: History::new(address),
            page,
            shell: Shell::new(),
            popup: None,
            shown_site_id: String::new(),
            following_link: false,
            canvas: new_canvas(width, height),
            scale: valid_scale(scale),
            maximized: false,
            requested_height: None,
            dirty: true,
            log,
            commands: Vec::new(),
        };
        viewer.page_started();
        viewer
    }

    /// The address showing: a path, a URL, or empty for the start page.
    pub fn address(&self) -> &str {
        self.history.current()
    }

    /// The window was resized, or moved to a display with another scale.
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) {
        self.canvas = new_canvas(width, height);
        self.scale = valid_scale(scale);
        self.lay_out();
    }

    /// The window was maximized or restored.
    pub fn set_maximized(&mut self, maximized: bool) {
        if self.maximized != maximized {
            self.maximized = maximized;
            self.requested_height = None;
        }
    }

    /// Opens `address` as a new history entry.
    pub fn open(&mut self, address: &str) {
        self.log.push(format!("navigate: {address}"));
        self.history.push(address);
        self.load();
    }

    /// Goes back one page, if there is one.
    pub fn back(&mut self) {
        if self.history.back() {
            self.load();
        }
    }

    /// Goes forward one page, if there is one.
    pub fn forward(&mut self) {
        if self.history.forward() {
            self.load();
        }
    }

    /// Loads the current page again, from the start.
    pub fn reload(&mut self) {
        self.load();
    }

    /// The main pointer button went down at `(x, y)` in device pixels.
    pub fn pointer_down(&mut self, x: f32, y: f32) {
        let (x, y) = (x / self.scale, y / self.scale);
        let size = self.viewport();
        if let Some(popup) = &mut self.popup {
            popup.press(x, y, size);
        } else if !self.shell.contains(x, y) {
            if self.shell.focused {
                self.focus_address(false);
            }
            self.page.press(x, y);
        }
        self.dirty = true;
    }

    /// The main pointer button came up at `(x, y)` in device pixels.
    pub fn pointer_up(&mut self, x: f32, y: f32) {
        let (x, y) = (x / self.scale, y / self.scale);
        let size = self.viewport();
        if let Some(popup) = &mut self.popup {
            if popup.release(x, y, size) {
                self.popup = None;
            }
            self.dirty = true;
            return;
        }
        if self.shell.contains(x, y) {
            let control = self.shell.control_at(x, y);
            if control != Some(Control::Address) && self.shell.focused {
                self.focus_address(false);
            }
            if let Some(control) = control {
                self.use_control(control);
            }
            return;
        }
        let href = self.page.release(x, y);
        self.apply_requests();
        if let Some(target) = href.and_then(|href| resolve_link(self.history.current(), &href)) {
            self.following_link = true;
            self.open(&target);
        }
    }

    /// Whether the window needs to show a new frame.
    pub fn needs_redraw(&self) -> bool {
        self.dirty
    }

    /// The window's contents, drawn again if anything changed.
    pub fn render(&mut self) -> &Pixmap {
        if self.dirty {
            let size = self.viewport();
            let mut layers = vec![self.page.display_list(), self.shell.display_list()];
            if let Some(popup) = &self.popup {
                layers.push(popup.display_list(size, &self.fonts));
            }
            let background = self.page.background().unwrap_or(DEFAULT_BACKGROUND);
            draw_frame(
                &mut self.canvas,
                background,
                &layers,
                &self.fonts,
                self.scale,
            );
            self.dirty = false;
        }
        &self.canvas
    }

    /// What happened since last asked, one line each: what was loaded and
    /// verified, errors, and the pages' `console.log` output.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }

    /// What the window should do, in order.
    pub fn take_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }

    fn use_control(&mut self, control: Control) {
        match control {
            Control::Back => self.back(),
            Control::Forward => self.forward(),
            Control::Reload => self.reload(),
            Control::Address => self.focus_address(true),
            Control::Minimize => self.commands.push(WindowCommand::Minimize),
            Control::Maximize => self.commands.push(WindowCommand::ToggleMaximize),
            Control::Close => self.commands.push(WindowCommand::Close),
        }
    }

    /// Gives the address field the keyboard, with its text selected, or takes it away.
    pub(crate) fn focus_address(&mut self, focused: bool) {
        self.shell.focused = focused;
        self.shell.select_all = focused;
        self.lay_out();
    }

    /// Loads the current history entry, replacing the page.
    fn load(&mut self) {
        let location = Location::parse(self.history.current());
        let loader = Loader {
            trust: &self.trust,
            store: &self.store,
            fetch: &*self.fetch,
            allow_unsigned: self.allow_unsigned,
        };
        let loaded = loader.load(&location, &mut self.log);
        self.popup = None;
        self.page = Page::start(loaded, &mut self.log);
        self.page_started();
    }

    /// Shows the page just started: its title, the shell and window it asks
    /// for, its layout, and what its scripts asked for.
    fn page_started(&mut self) {
        let prefs = WindowPrefs::parse(&self.page.window);
        // Following a link into another site shows the full shell, so the
        // user sees where they landed, whatever the new site asked for. A
        // site opened directly starts the way it asked.
        let crossed = !self.shown_site_id.is_empty() && self.page.site_id != self.shown_site_id;
        if std::mem::take(&mut self.following_link)
            && crossed
            && (!prefs.title_bar || !prefs.address_bar)
        {
            self.shell.forced = true;
        }
        self.shown_site_id.clone_from(&self.page.site_id);

        self.commands
            .push(WindowCommand::SetTitle(self.page.title.clone()));
        self.commands
            .push(WindowCommand::SetResizable(prefs.resizable));
        // The system rounds the corners, and never while maximized.
        self.commands
            .push(WindowCommand::SetRoundedCorners(prefs.corner_radius > 0.0));
        if prefs.width.is_some() || prefs.height.is_some() {
            self.commands.push(WindowCommand::SetSize {
                width: prefs.width,
                height: prefs.height,
            });
        }
        self.requested_height = None;

        self.shell.set_prefs(prefs);
        self.shell.address = self.history.current().to_owned();
        self.shell.can_go_back = self.history.can_go_back();
        self.shell.can_go_forward = self.history.can_go_forward();
        self.shell.focused = false;
        self.shell.select_all = false;
        self.lay_out();
        self.apply_requests();
    }

    /// Lays out the shell, then the page below it.
    pub(crate) fn lay_out(&mut self) {
        let (width, _) = self.viewport();
        self.shell.lay_out(&self.fonts, width);
        self.page.lay_out(&self.fonts, width, self.shell.height());
        self.fit_height();
        self.dirty = true;
    }

    /// `"height": "auto"`: asks the window to wrap the shell and page.
    fn fit_height(&mut self) {
        if !self.shell.prefs().auto_height || self.maximized {
            return;
        }
        let wanted = (self.shell.height() + self.page.height()).max(MIN_AUTO_HEIGHT);
        let (_, current) = self.viewport();
        let changed = (wanted - current).abs() * self.scale > 1.0;
        if changed && self.requested_height != Some(wanted) {
            self.requested_height = Some(wanted);
            self.commands.push(WindowCommand::SetSize {
                width: None,
                height: Some(wanted),
            });
        }
    }

    /// Acts on what the page's scripts asked for. A changed page is laid
    /// out again at once, so input never meets a layout of an older page.
    fn apply_requests(&mut self) {
        for request in self.page.take_requests() {
            match request {
                HostRequest::Log(line) => self.log.push(format!("console: {line}")),
                HostRequest::Popup(message) => self.popup = Some(Popup::new(message)),
                HostRequest::Close => self.commands.push(WindowCommand::Close),
                HostRequest::Minimize => self.commands.push(WindowCommand::Minimize),
                HostRequest::Maximize => self.commands.push(WindowCommand::ToggleMaximize),
                HostRequest::Invalidate => self.lay_out(),
            }
        }
        self.dirty = true;
    }

    /// Marks the canvas out of date.
    pub(crate) fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// The window's size in CSS pixels.
    pub(crate) fn viewport(&self) -> (f32, f32) {
        (
            self.canvas.width() as f32 / self.scale,
            self.canvas.height() as f32 / self.scale,
        )
    }
}

/// A canvas `width` × `height`, at least one pixel each way: a minimized
/// window reports zero.
fn new_canvas(width: u32, height: u32) -> Pixmap {
    Pixmap::new(width.max(1), height.max(1)).expect("a positive size")
}

fn valid_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

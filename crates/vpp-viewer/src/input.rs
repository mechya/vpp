//! Keyboard input, the address field, and the parts of the window that move
//! or resize it (`docs/reference/viewer.md`, "The shell").
//!
//! The window is frameless: the viewer draws its own bar. So the viewer also
//! says where the window can be dragged and resized, and the entry point asks
//! the system to do it.

use crate::app::{Viewer, WindowCommand};
use crate::navigation::typed_address;

/// How far in from the window's edges the resize areas reach, in CSS pixels.
const RESIZE_BORDER: f32 = 6.0;
/// With the bar hidden, the top of the page drags the window, this far down, in CSS pixels.
const DRAG_STRIP: f32 = 40.0;

/// A key the viewer acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// Esc.
    Escape,
    /// Enter.
    Enter,
    /// Backspace.
    Backspace,
    /// F5.
    F5,
    /// The left arrow.
    ArrowLeft,
    /// The right arrow.
    ArrowRight,
    /// A keyboard's or mouse's back key.
    BrowserBack,
    /// A keyboard's or mouse's forward key.
    BrowserForward,
    /// A keyboard's refresh key.
    BrowserRefresh,
    /// Any other key, by the character it types without modifiers, such as `"l"`.
    Character(String),
}

/// The modifier keys held down.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Ctrl.
    pub ctrl: bool,
    /// Alt.
    pub alt: bool,
    /// Shift.
    pub shift: bool,
}

impl Modifiers {
    fn none(self) -> bool {
        self == Self::default()
    }

    fn only_ctrl(self) -> bool {
        self == Self {
            ctrl: true,
            ..Self::default()
        }
    }

    fn only_alt(self) -> bool {
        self == Self {
            alt: true,
            ..Self::default()
        }
    }
}

/// What a point in the window does when the main button goes down there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRegion {
    /// Goes to the page, the shell, or the popup, through [`Viewer::pointer_down`].
    Content,
    /// Moves the window.
    Drag,
    /// Resizes the window from this edge or corner.
    Resize(ResizeEdge),
}

/// An edge or corner of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    /// The top edge.
    North,
    /// The bottom edge.
    South,
    /// The right edge.
    East,
    /// The left edge.
    West,
    /// The top-right corner.
    NorthEast,
    /// The top-left corner.
    NorthWest,
    /// The bottom-right corner.
    SouthEast,
    /// The bottom-left corner.
    SouthWest,
}

impl Viewer {
    /// Whether the address field has the keyboard. Key repeat is for typing only.
    pub fn is_editing_address(&self) -> bool {
        self.shell.focused
    }

    /// A key went down.
    ///
    /// In the address field: Enter opens the address, Esc puts back the
    /// current one, Backspace deletes, Ctrl+A and Ctrl+L select all.
    /// Elsewhere: Esc closes the popup, then the revealed shell, then the
    /// window; Ctrl+L reveals the shell and edits the address; R, F5, and the
    /// refresh key reload; Alt+Left, Alt+Right, and the back and forward keys
    /// move through history.
    pub fn key_down(&mut self, key: Key, modifiers: Modifiers) {
        if self.shell.focused {
            self.edit_address(&key, modifiers);
            return;
        }
        let letter =
            |wanted: &str| matches!(&key, Key::Character(c) if c.eq_ignore_ascii_case(wanted));
        match key {
            Key::Escape if self.popup.is_some() => {
                self.popup = None;
                self.invalidate();
            }
            Key::Escape if self.shell.forced => {
                self.shell.forced = false;
                self.lay_out();
            }
            Key::Escape => self.commands.push(WindowCommand::Close),
            _ if modifiers.only_ctrl() && letter("l") => {
                self.shell.forced = true;
                self.focus_address(true);
            }
            Key::F5 | Key::BrowserRefresh => self.reload(),
            _ if modifiers.none() && letter("r") => self.reload(),
            Key::BrowserBack => self.back(),
            Key::ArrowLeft if modifiers.only_alt() => self.back(),
            Key::BrowserForward => self.forward(),
            Key::ArrowRight if modifiers.only_alt() => self.forward(),
            _ => {}
        }
    }

    /// Text typed on the keyboard. Only the address field takes it.
    pub fn text_input(&mut self, text: &str) {
        if !self.shell.focused {
            return;
        }
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        if text.is_empty() {
            return;
        }
        if self.shell.select_all {
            self.shell.address.clear();
            self.shell.select_all = false;
        }
        self.shell.address.push_str(&text);
        self.lay_out();
    }

    /// Ctrl+V: `text` from the clipboard goes into the address field, as if
    /// typed, with line breaks removed.
    pub fn paste(&mut self, text: &str) {
        self.text_input(text);
    }

    /// What the main button does at `(x, y)` in device pixels.
    pub fn region_at(&self, x: f32, y: f32) -> WindowRegion {
        let (x, y) = (x / self.scale, y / self.scale);
        if self.popup.is_some() {
            return WindowRegion::Content;
        }
        if self.shell.prefs().resizable && !self.maximized {
            if let Some(edge) = self.edge_at(x, y) {
                return WindowRegion::Resize(edge);
            }
        }
        if self.shell.contains(x, y) {
            return match self.shell.control_at(x, y) {
                Some(_) => WindowRegion::Content,
                None => WindowRegion::Drag,
            };
        }
        if !self.shell.visible() && y < DRAG_STRIP && !self.page.is_control_at(x, y) {
            return WindowRegion::Drag;
        }
        WindowRegion::Content
    }

    fn edge_at(&self, x: f32, y: f32) -> Option<ResizeEdge> {
        let (width, height) = self.viewport();
        let left = x < RESIZE_BORDER;
        let right = x > width - RESIZE_BORDER;
        let top = y < RESIZE_BORDER;
        let bottom = y > height - RESIZE_BORDER;
        Some(match (top, bottom, left, right) {
            (true, _, true, _) => ResizeEdge::NorthWest,
            (true, _, _, true) => ResizeEdge::NorthEast,
            (_, true, true, _) => ResizeEdge::SouthWest,
            (_, true, _, true) => ResizeEdge::SouthEast,
            (true, ..) => ResizeEdge::North,
            (_, true, ..) => ResizeEdge::South,
            (_, _, true, _) => ResizeEdge::West,
            (_, _, _, true) => ResizeEdge::East,
            _ => return None,
        })
    }

    fn edit_address(&mut self, key: &Key, modifiers: Modifiers) {
        match key {
            Key::Escape => {
                self.shell.address = self.history.current().to_owned();
                self.shell.forced = false;
                self.focus_address(false);
            }
            Key::Enter => self.submit_address(),
            Key::Backspace => {
                if self.shell.select_all {
                    self.shell.address.clear();
                } else {
                    self.shell.address.pop();
                }
                self.shell.select_all = false;
                self.lay_out();
            }
            Key::Character(c)
                if modifiers.only_ctrl()
                    && (c.eq_ignore_ascii_case("a") || c.eq_ignore_ascii_case("l")) =>
            {
                self.shell.select_all = true;
            }
            _ => {}
        }
    }

    /// Opens what was typed. An empty field just gives the keyboard back.
    fn submit_address(&mut self) {
        let target = typed_address(&self.shell.address);
        self.shell.forced = false;
        self.focus_address(false);
        if target.is_empty() {
            self.shell.address = self.history.current().to_owned();
            self.lay_out();
        } else {
            self.open(&target);
        }
    }
}

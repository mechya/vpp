//! The shell: the viewer's own bar above the page, drawn by the VPP engine.
//!
//! Back, forward, reload, the address field, and the window buttons. It is a
//! separate document with its own stylesheet, so pages cannot style it. A
//! site chooses only its colour and which parts show (`WindowPrefs`); the
//! colour is parsed first and written back as a colour, never as raw CSS.

use vpp_dom::{Document, NodeId, parse_html};
use vpp_layout::{LayoutBox, Rect, layout_document};
use vpp_paint::{DisplayItem, FontSet, build_display_list};
use vpp_style::values::color::parse_color;
use vpp_style::{Color, StyleSheet, parse_stylesheet};

use crate::window_prefs::WindowPrefs;

/// The shell page. ASCII labels: the text renderer does not shape other scripts yet.
const HTML: &str = r#"<html><body><div id="bar">
<button id="back" class="nav">&lt;</button>
<button id="forward" class="nav">&gt;</button>
<button id="reload" class="nav">R</button>
<div id="address"><div id="address-text"></div></div>
<button id="minimize" class="win">_</button>
<button id="maximize" class="win">[ ]</button>
<button id="close" class="win close">X</button>
</div></body></html>"#;

/// The stylesheet, with `THEME`, `BUTTON`, `TEXT`, `MUTED`, and `FIELD`
/// replaced by colours that suit the theme.
const CSS: &str = "
body { margin: 0; padding: 0; }
#bar { display: flex; align-items: center; gap: 6px; padding: 6px 8px; background-color: THEME; }
button.nav, button.win { padding: 4px 10px; font-size: 13px; border-radius: 6px;
  background-color: BUTTON; color: TEXT; font-weight: bold; }
button.disabled { color: MUTED; }
button.close { background-color: #dc2626; color: white; }
#address { flex: 1; padding: 5px 10px; background-color: FIELD; border: 2px solid FIELD; border-radius: 6px; }
#address.focused { border: 2px solid #2563eb; }
#address.hidden { display: none; }
#address-text { font-size: 13px; color: #18181b; }
";

const DEFAULT_THEME: Color = Color::rgb(39, 39, 42);
const PLACEHOLDER: &str = "Enter a page address: a .vpp file or URL";
/// The address text's size, as in `CSS`.
const TEXT_SIZE: f32 = 13.0;

/// A control in the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    Back,
    Forward,
    Reload,
    Address,
    Minimize,
    Maximize,
    Close,
}

impl Control {
    fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "back" => Self::Back,
            "forward" => Self::Forward,
            "reload" => Self::Reload,
            "address" => Self::Address,
            "minimize" => Self::Minimize,
            "maximize" => Self::Maximize,
            "close" => Self::Close,
            _ => return None,
        })
    }
}

/// The shell and its state.
pub(crate) struct Shell {
    prefs: WindowPrefs,
    sheet: StyleSheet,
    document: Document,
    layout: Option<LayoutBox>,
    /// Shown in full whatever the site asked for: after Ctrl+L, or after
    /// following a link into another site. Esc ends it.
    pub(crate) forced: bool,
    /// The text in the address field.
    pub(crate) address: String,
    /// Whether the address field has the keyboard.
    pub(crate) focused: bool,
    /// Whether the next typed text replaces the whole address.
    pub(crate) select_all: bool,
    pub(crate) can_go_back: bool,
    pub(crate) can_go_forward: bool,
}

impl Shell {
    pub(crate) fn new() -> Self {
        let prefs = WindowPrefs::default();
        Self {
            sheet: stylesheet(&prefs),
            prefs,
            document: Document::new(),
            layout: None,
            forced: false,
            address: String::new(),
            focused: false,
            select_all: false,
            can_go_back: false,
            can_go_forward: false,
        }
    }

    pub(crate) fn prefs(&self) -> &WindowPrefs {
        &self.prefs
    }

    /// Uses a new page's preferences.
    pub(crate) fn set_prefs(&mut self, prefs: WindowPrefs) {
        self.sheet = stylesheet(&prefs);
        self.prefs = prefs;
    }

    pub(crate) fn visible(&self) -> bool {
        self.prefs.title_bar || self.forced
    }

    /// The bar's height in CSS pixels: 0 while hidden.
    pub(crate) fn height(&self) -> f32 {
        self.layout.as_ref().map_or(0.0, |root| root.frame.h)
    }

    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        self.layout
            .as_ref()
            .is_some_and(|root| root.frame.contains(x, y))
    }

    /// The control under a point in CSS pixels, if any.
    pub(crate) fn control_at(&self, x: f32, y: f32) -> Option<Control> {
        let node = self.layout.as_ref()?.hit_test(x, y)?.node?;
        std::iter::successors(Some(node), |&n| self.document[n].parent())
            .find_map(|n| Control::from_id(self.document.element(n)?.id()?))
    }

    /// Builds the bar from the current state and lays it out `width` CSS pixels wide.
    pub(crate) fn lay_out(&mut self, fonts: &FontSet, width: f32) {
        if !self.visible() {
            self.layout = None;
            return;
        }
        let mut document = parse_html(HTML);
        let field_class = if !self.prefs.address_bar && !self.forced {
            "hidden"
        } else if self.focused {
            "focused"
        } else {
            ""
        };
        set_class(&mut document, "address", field_class);
        set_class(&mut document, "back", nav_class(self.can_go_back));
        set_class(&mut document, "forward", nav_class(self.can_go_forward));

        let text = if self.address.is_empty() && !self.focused {
            PLACEHOLDER.to_owned()
        } else {
            self.address.clone()
        };
        let caret = if self.focused { "|" } else { "" };
        // Set as text, never parsed: an address cannot inject markup.
        let node = document
            .find_by_id(document.root(), "address-text")
            .expect("the shell has an address field");
        let sheets = std::slice::from_ref(&self.sheet);

        // First with the field empty, to find how wide it is: a long address
        // must not push the window buttons out of the window.
        document.set_text_content(node, caret);
        let room = layout_document(&document, sheets, fonts, width)
            .and_then(|root| box_of(&root, node))
            .map_or(0.0, |field| field.w - fonts.regular.width(caret, TEXT_SIZE));
        document.set_text_content(node, format!("{}{caret}", fit(&text, room, fonts)));

        self.layout = layout_document(&document, sheets, fonts, width);
        self.document = document;
    }

    /// The border box of the shell element with `id`, for tests to click on.
    #[cfg(test)]
    pub(crate) fn box_of(&self, id: &str) -> Option<Rect> {
        let node = self.document.find_by_id(self.document.root(), id)?;
        box_of(self.layout.as_ref()?, node)
    }

    /// What the bar draws, in CSS pixels.
    pub(crate) fn display_list(&self) -> Vec<DisplayItem> {
        match &self.layout {
            Some(root) => build_display_list(root, &self.document),
            None => Vec::new(),
        }
    }
}

/// The border box of `node` in a laid-out shell.
fn box_of(b: &LayoutBox, node: NodeId) -> Option<Rect> {
    if b.node == Some(node) {
        return Some(b.frame);
    }
    b.children.iter().find_map(|c| box_of(c, node))
}

/// `text` if it fits in `room` CSS pixels, otherwise its end after "...":
/// the end of an address says most about where it leads.
fn fit(text: &str, room: f32, fonts: &FontSet) -> String {
    let font = &fonts.regular;
    if font.width(text, TEXT_SIZE) <= room {
        return text.to_owned();
    }
    let mut start = 0;
    for (i, _) in text.char_indices().skip(1) {
        start = i;
        if font.width(&format!("...{}", &text[i..]), TEXT_SIZE) <= room {
            break;
        }
    }
    format!("...{}", &text[start..])
}

fn nav_class(enabled: bool) -> &'static str {
    if enabled { "nav" } else { "nav disabled" }
}

fn set_class(document: &mut Document, id: &str, class: &str) {
    let node = document
        .find_by_id(document.root(), id)
        .expect("the shell has every control");
    if let Some(element) = document.element_mut(node) {
        element.set_attribute("class", class);
    }
}

/// The shell stylesheet in the site's colour, with text and buttons light
/// on a dark theme and dark on a light one.
fn stylesheet(prefs: &WindowPrefs) -> StyleSheet {
    let theme = prefs
        .theme
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(DEFAULT_THEME);
    let dark = luminance(theme) < 0.5;
    let pick =
        |on_dark: &'static str, on_light: &'static str| if dark { on_dark } else { on_light };
    let css = CSS
        .replace("THEME", &css_color(theme))
        .replace("BUTTON", pick("rgba(255,255,255,0.14)", "rgba(0,0,0,0.08)"))
        .replace("MUTED", pick("rgba(255,255,255,0.35)", "rgba(0,0,0,0.3)"))
        .replace("FIELD", pick("#f4f4f5", "#ffffff"))
        .replace("TEXT", pick("#ffffff", "#18181b"));
    parse_stylesheet(&css)
}

fn css_color(c: Color) -> String {
    format!("rgba({},{},{},{})", c.r, c.g, c.b, f32::from(c.a) / 255.0)
}

/// Relative luminance, 0 (black) to 1 (white).
fn luminance(c: Color) -> f32 {
    (0.2126 * f32::from(c.r) + 0.7152 * f32::from(c.g) + 0.0722 * f32::from(c.b)) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_is_a_colour_or_nothing() {
        // INTENTIONAL: hostile input. A theme that tries to add CSS rules must not reach the stylesheet.
        let prefs = WindowPrefs {
            theme: Some("red; } #address { display: none".into()),
            ..WindowPrefs::default()
        };
        assert_eq!(stylesheet(&prefs), stylesheet(&WindowPrefs::default()));

        let prefs = WindowPrefs {
            theme: Some("#2563EB".into()),
            ..WindowPrefs::default()
        };
        assert_ne!(stylesheet(&prefs), stylesheet(&WindowPrefs::default()));
    }

    #[test]
    fn dark_themes_get_light_text() {
        assert!(luminance(DEFAULT_THEME) < 0.5);
        assert!(luminance(Color::rgb(255, 255, 255)) > 0.5);
    }
}

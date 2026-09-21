//! The message box a page opens with `VPP.window.popup(message)`: a card
//! centred over a dimmed page, with an OK button. It is drawn by the viewer,
//! so a page cannot make it look like anything else.

use vpp_layout::Rect;
use vpp_paint::{DisplayItem, FontSet};
use vpp_style::Color;

const WIDTH: f32 = 340.0;
const HEIGHT: f32 = 180.0;
const BUTTON_WIDTH: f32 = 90.0;
const BUTTON_HEIGHT: f32 = 40.0;
/// The button's distance from the card's right and bottom edges, to its far corner.
const BUTTON_INSET: f32 = 20.0;
const TEXT_SIZE: f32 = 18.0;

const OVERLAY: Color = Color::rgba(0, 0, 0, 110);
const SHADOW: Color = Color::rgba(0, 0, 0, 40);
const CARD: Color = Color::rgb(255, 255, 255);
const TEXT: Color = Color::rgb(24, 24, 27);
const BUTTON: Color = Color::rgb(37, 99, 235);
const BUTTON_DOWN: Color = Color::rgb(29, 78, 216);
const BUTTON_TEXT: Color = Color::rgb(255, 255, 255);

/// An open popup. While one is open, the page gets no input.
#[derive(Debug)]
pub(crate) struct Popup {
    message: String,
    /// Whether the pointer went down on OK and has not come up yet.
    pressed: bool,
}

impl Popup {
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            pressed: false,
        }
    }

    /// The pointer went down at a point in CSS pixels, in a viewport `size` large.
    pub(crate) fn press(&mut self, x: f32, y: f32, size: (f32, f32)) {
        self.pressed = button(card(size)).contains(x, y);
    }

    /// The pointer went up. `true` if that clicked OK, which closes the popup.
    pub(crate) fn release(&mut self, x: f32, y: f32, size: (f32, f32)) -> bool {
        let clicked = self.pressed && button(card(size)).contains(x, y);
        self.pressed = false;
        clicked
    }

    /// What the popup draws over the page, in CSS pixels.
    pub(crate) fn display_list(&self, size: (f32, f32), fonts: &FontSet) -> Vec<DisplayItem> {
        let card = card(size);
        let button = button(card);
        let message_area = Rect::new(
            card.x,
            card.y,
            card.w,
            card.h - BUTTON_HEIGHT - BUTTON_INSET,
        );
        let mut items = vec![
            fill(Rect::new(0.0, 0.0, size.0, size.1), 0.0, OVERLAY),
            fill(card.translated(0.0, 6.0), 12.0, SHADOW),
            fill(card, 12.0, CARD),
            fill(button, 8.0, if self.pressed { BUTTON_DOWN } else { BUTTON }),
        ];
        items.push(centered_text(fonts, message_area, &self.message, TEXT));
        items.push(centered_text(fonts, button, "OK", BUTTON_TEXT));
        items
    }
}

fn card((width, height): (f32, f32)) -> Rect {
    Rect::new(
        (width - WIDTH) / 2.0,
        (height - HEIGHT) / 2.0,
        WIDTH,
        HEIGHT,
    )
}

fn button(card: Rect) -> Rect {
    Rect::new(
        card.x + card.w - BUTTON_INSET - BUTTON_WIDTH,
        card.y + card.h - BUTTON_INSET - BUTTON_HEIGHT,
        BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn fill(rect: Rect, radius: f32, color: Color) -> DisplayItem {
    DisplayItem::Fill {
        rect,
        radius,
        color,
    }
}

/// One line of text centred in `area`.
fn centered_text(fonts: &FontSet, area: Rect, text: &str, color: Color) -> DisplayItem {
    let font = &fonts.regular;
    let width = font.width(text, TEXT_SIZE);
    DisplayItem::Text {
        x: area.x + (area.w - width) / 2.0,
        baseline: area.y + (area.h - font.line_height(TEXT_SIZE)) / 2.0 + font.ascent(TEXT_SIZE),
        text: text.to_owned(),
        font_size: TEXT_SIZE,
        bold: false,
        color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: (f32, f32) = (800.0, 520.0);

    #[test]
    fn ok_needs_down_and_up_on_the_button() {
        let ok = button(card(SIZE));
        let (x, y) = (ok.x + 1.0, ok.y + 1.0);
        let mut popup = Popup::new("hi".into());

        popup.press(x, y, SIZE);
        assert!(popup.release(x, y, SIZE));

        popup.press(0.0, 0.0, SIZE);
        assert!(!popup.release(x, y, SIZE));

        popup.press(x, y, SIZE);
        assert!(!popup.release(0.0, 0.0, SIZE));
    }
}

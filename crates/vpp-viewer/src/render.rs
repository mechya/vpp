//! Drawing one frame of the window's contents.

use vpp_layout::Rect;
use vpp_paint::{DisplayItem, FontSet, Pixmap, rasterize};
use vpp_style::Color;

/// Behind a page with no background of its own.
pub(crate) const DEFAULT_BACKGROUND: Color = Color::rgb(244, 244, 245);

/// Clears `pixmap` to `background` and draws `layers` over it in order.
/// Positions are CSS pixels; `scale` is device pixels per CSS pixel.
pub(crate) fn draw_frame(
    pixmap: &mut Pixmap,
    background: Color,
    layers: &[Vec<DisplayItem>],
    fonts: &FontSet,
    scale: f32,
) {
    let whole = Rect::new(
        0.0,
        0.0,
        pixmap.width() as f32 / scale,
        pixmap.height() as f32 / scale,
    );
    let mut clear = vec![DisplayItem::Fill {
        rect: whole,
        radius: 0.0,
        color: DEFAULT_BACKGROUND,
    }];
    if background != DEFAULT_BACKGROUND {
        clear.push(DisplayItem::Fill {
            rect: whole,
            radius: 0.0,
            color: background,
        });
    }
    let mut target = pixmap.as_mut();
    rasterize(&clear, &mut target, fonts, scale);
    for items in layers {
        rasterize(items, &mut target, fonts, scale);
    }
}

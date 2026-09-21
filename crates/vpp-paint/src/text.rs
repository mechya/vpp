//! Drawing text: each character's glyph outline from `swash`, filled with
//! `tiny-skia`. Characters advance by the same widths layout measured
//! (`font.rs`), so text always fits the space it was given.

use resvg::tiny_skia::{FillRule, Paint, PathBuilder, PixmapMut, Transform};
use swash::scale::ScaleContext;
use swash::zeno::{Command, PathData};
use vpp_style::Color;

use crate::font::Font;

/// Draws `text` with its left edge at `x` and baseline at `baseline`, both
/// in CSS pixels, at `size` CSS pixels, scaled by `scale` to device pixels.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text(
    pixmap: &mut PixmapMut<'_>,
    context: &mut ScaleContext,
    font: &Font,
    text: &str,
    x: f32,
    baseline: f32,
    size: f32,
    color: Color,
    scale: f32,
) {
    let face = font.face();
    let charmap = face.charmap();
    let advances = face.glyph_metrics(&[]).scale(size * scale);
    let mut scaler = context.builder(face).size(size * scale).hint(false).build();

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;

    let mut pen = x * scale;
    let base = baseline * scale;
    for c in text.chars() {
        let glyph = charmap.map(c);
        if let Some(outline) = scaler.scale_outline(glyph) {
            let mut path = PathBuilder::new();
            // Font outlines point y upwards; the page's y points down.
            for command in outline.path().commands() {
                match command {
                    Command::MoveTo(p) => path.move_to(pen + p.x, base - p.y),
                    Command::LineTo(p) => path.line_to(pen + p.x, base - p.y),
                    Command::QuadTo(c, p) => {
                        path.quad_to(pen + c.x, base - c.y, pen + p.x, base - p.y)
                    }
                    Command::CurveTo(c1, c2, p) => path.cubic_to(
                        pen + c1.x,
                        base - c1.y,
                        pen + c2.x,
                        base - c2.y,
                        pen + p.x,
                        base - p.y,
                    ),
                    Command::Close => path.close(),
                }
            }
            if let Some(path) = path.finish() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
        pen += advances.advance_width(glyph);
    }
}

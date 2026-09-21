//! Drawing a display list into pixels with `tiny-skia`.

use resvg::tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, PixmapMut, Transform};
use swash::scale::ScaleContext;
use vpp_dom::Document;
use vpp_layout::{LayoutBox, Rect};
use vpp_style::Color;

use crate::display_list::{DisplayItem, build_display_list};
use crate::font::FontSet;
use crate::svg::draw_svg;
use crate::text::draw_text;

/// Draws `items` into `pixmap`. Positions are CSS pixels; `scale` is device
/// pixels per CSS pixel (2 on a 200% display).
pub fn rasterize(items: &[DisplayItem], pixmap: &mut PixmapMut<'_>, fonts: &FontSet, scale: f32) {
    let mut context = ScaleContext::new();
    for item in items {
        match item {
            DisplayItem::Fill {
                rect,
                radius,
                color,
            } => {
                if let Some(path) = rounded_rect(rect.scaled(scale), radius * scale) {
                    fill(pixmap, &path, *color, FillRule::Winding);
                }
            }
            DisplayItem::Border {
                rect,
                radius,
                width,
                color,
            } => {
                // The ring between the outer edge and the inner edge.
                let outer = rect.scaled(scale);
                let w = (width * scale).max(1.0);
                let inner = Rect::new(
                    outer.x + w,
                    outer.y + w,
                    outer.w - 2.0 * w,
                    outer.h - 2.0 * w,
                );
                let mut ring = PathBuilder::new();
                push_rounded_rect(&mut ring, outer, radius * scale);
                if inner.w > 0.0 && inner.h > 0.0 {
                    push_rounded_rect(&mut ring, inner, (radius * scale - w).max(0.0));
                }
                if let Some(path) = ring.finish() {
                    fill(pixmap, &path, *color, FillRule::EvenOdd);
                }
            }
            DisplayItem::Text {
                x,
                baseline,
                text,
                font_size,
                bold,
                color,
            } => draw_text(
                pixmap,
                &mut context,
                fonts.pick(*bold),
                text,
                *x,
                *baseline,
                *font_size,
                *color,
                scale,
            ),
            DisplayItem::Svg { rect, source } => draw_svg(pixmap, source, *rect, scale),
        }
    }
}

/// Renders a laid-out page onto white, `scale` device pixels per CSS pixel,
/// as large as the page.
pub fn render_page(root: &LayoutBox, document: &Document, fonts: &FontSet, scale: f32) -> Pixmap {
    let width = (root.frame.x + root.frame.w).max(1.0);
    let height = (root.frame.y + root.frame.h).max(1.0);
    let mut pixmap = Pixmap::new(
        (width * scale).ceil() as u32,
        (height * scale).ceil() as u32,
    )
    .expect("a page has a positive size");
    pixmap.fill(tiny_skia::Color::WHITE);
    let items = build_display_list(root, document);
    rasterize(&items, &mut pixmap.as_mut(), fonts, scale);
    pixmap
}

fn fill(pixmap: &mut PixmapMut<'_>, path: &tiny_skia::Path, color: Color, rule: FillRule) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, rule, Transform::identity(), None);
}

fn rounded_rect(rect: Rect, radius: f32) -> Option<tiny_skia::Path> {
    let mut path = PathBuilder::new();
    push_rounded_rect(&mut path, rect, radius);
    path.finish()
}

/// Adds a rectangle with circular corners, clamped so opposite corners never overlap.
fn push_rounded_rect(path: &mut PathBuilder, r: Rect, radius: f32) {
    if r.w <= 0.0 || r.h <= 0.0 {
        return;
    }
    let radius = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    let (left, top, right, bottom) = (r.x, r.y, r.x + r.w, r.y + r.h);
    if radius == 0.0 {
        path.push_rect(
            tiny_skia::Rect::from_ltrb(left, top, right, bottom).expect("positive size"),
        );
        return;
    }
    // Control-point distance for a quarter circle drawn as one cubic curve.
    let k = radius * 0.552_284_8;
    path.move_to(left + radius, top);
    path.line_to(right - radius, top);
    path.cubic_to(
        right - radius + k,
        top,
        right,
        top + radius - k,
        right,
        top + radius,
    );
    path.line_to(right, bottom - radius);
    path.cubic_to(
        right,
        bottom - radius + k,
        right - radius + k,
        bottom,
        right - radius,
        bottom,
    );
    path.line_to(left + radius, bottom);
    path.cubic_to(
        left + radius - k,
        bottom,
        left,
        bottom - radius + k,
        left,
        bottom - radius,
    );
    path.line_to(left, top + radius);
    path.cubic_to(
        left,
        top + radius - k,
        left + radius - k,
        top,
        left + radius,
        top,
    );
    path.close();
}

trait Scaled {
    fn scaled(self, scale: f32) -> Self;
}

impl Scaled for Rect {
    fn scaled(self, scale: f32) -> Self {
        Rect::new(
            self.x * scale,
            self.y * scale,
            self.w * scale,
            self.h * scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::tests::test_fonts;

    fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        let p = pixmap.pixel(x, y).unwrap();
        [p.red(), p.green(), p.blue(), p.alpha()]
    }

    fn render(items: &[DisplayItem], width: u32, height: u32, scale: f32) -> Pixmap {
        let mut pixmap = Pixmap::new(width, height).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);
        rasterize(items, &mut pixmap.as_mut(), &test_fonts(), scale);
        pixmap
    }

    #[test]
    fn fills_rectangles_and_rounds_corners() {
        let items = [DisplayItem::Fill {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            radius: 8.0,
            color: Color::rgb(255, 0, 0),
        }];
        let p = render(&items, 20, 20, 1.0);
        assert_eq!(pixel(&p, 10, 10), [255, 0, 0, 255]);
        // The very corner is outside the rounded shape.
        assert_eq!(pixel(&p, 0, 0), [255, 255, 255, 255]);
    }

    #[test]
    fn borders_are_rings() {
        let items = [DisplayItem::Border {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            radius: 0.0,
            width: 2.0,
            color: Color::rgb(0, 0, 255),
        }];
        let p = render(&items, 20, 20, 1.0);
        assert_eq!(pixel(&p, 1, 10), [0, 0, 255, 255]);
        assert_eq!(pixel(&p, 10, 10), [255, 255, 255, 255]);
    }

    #[test]
    fn scale_doubles_everything() {
        let items = [DisplayItem::Fill {
            rect: Rect::new(5.0, 5.0, 5.0, 5.0),
            radius: 0.0,
            color: Color::rgb(0, 128, 0),
        }];
        let p = render(&items, 20, 20, 2.0);
        assert_eq!(pixel(&p, 12, 12), [0, 128, 0, 255]);
        assert_eq!(pixel(&p, 8, 8), [255, 255, 255, 255]);
    }

    #[test]
    fn text_leaves_ink_on_the_baseline() {
        let items = [DisplayItem::Text {
            x: 2.0,
            baseline: 20.0,
            text: "Hi".into(),
            font_size: 20.0,
            bold: false,
            color: Color::rgb(0, 0, 0),
        }];
        let p = render(&items, 40, 30, 1.0);
        let dark = (0..40)
            .flat_map(|x| (0..30).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel(&p, x, y)[0] < 100)
            .count();
        assert!(dark > 30, "{dark} dark pixels");
        // Nothing below the baseline for "Hi".
        assert!((0..40).all(|x| pixel(&p, x, 25)[0] > 200));
    }

    #[test]
    fn svg_icons_take_the_css_colour() {
        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 1 1" color="#ff0000" fill="currentColor"><path d="M0 0h1v1H0z"/></svg>"##;
        let items = [DisplayItem::Svg {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source: source.into(),
        }];
        let p = render(&items, 10, 10, 1.0);
        assert_eq!(pixel(&p, 5, 5), [255, 0, 0, 255]);
    }

    #[test]
    fn broken_svg_draws_nothing() {
        let items = [DisplayItem::Svg {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source: "<svg <<".into(),
        }];
        let p = render(&items, 10, 10, 1.0);
        assert_eq!(pixel(&p, 5, 5), [255, 255, 255, 255]);
    }
}

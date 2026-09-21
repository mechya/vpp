//! The display list: everything a page draws, in order, in CSS pixels.
//!
//! Building it walks the layout tree once; drawing it needs no DOM and no
//! layout. It is the seam where a GPU backend could replace the CPU
//! rasteriser (`docs/rust-port.md` §9).

use vpp_dom::Document;
use vpp_layout::{FragmentContent, LayoutBox, Rect};
use vpp_style::Color;

use crate::svg::svg_source;

/// One drawing operation.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayItem {
    /// A filled rectangle, with rounded corners if `radius` is above 0.
    Fill {
        /// The rectangle.
        rect: Rect,
        /// The corner radius.
        radius: f32,
        /// The colour.
        color: Color,
    },
    /// A border inside `rect`, `width` wide.
    Border {
        /// The border box.
        rect: Rect,
        /// The outer corner radius.
        radius: f32,
        /// The border width.
        width: f32,
        /// The colour.
        color: Color,
    },
    /// A word of text.
    Text {
        /// The left edge.
        x: f32,
        /// The baseline.
        baseline: f32,
        /// The text.
        text: String,
        /// The font size.
        font_size: f32,
        /// Whether it is bold.
        bold: bool,
        /// The colour.
        color: Color,
    },
    /// An SVG image, drawn to fit `rect`.
    Svg {
        /// The content box it is drawn in.
        rect: Rect,
        /// A standalone SVG document.
        source: String,
    },
}

/// The display list for a laid-out page.
pub fn build_display_list(root: &LayoutBox, document: &Document) -> Vec<DisplayItem> {
    let mut items = Vec::new();
    add_box(root, document, &mut items);
    items
}

fn add_box(b: &LayoutBox, document: &Document, items: &mut Vec<DisplayItem>) {
    let s = &b.style;
    if s.has_background() {
        items.push(DisplayItem::Fill {
            rect: b.frame,
            radius: s.border_radius,
            color: s.background,
        });
    }
    if s.border_width > 0.0 && s.border_color.a > 0 {
        items.push(DisplayItem::Border {
            rect: b.frame,
            radius: s.border_radius,
            width: s.border_width,
            color: s.border_color,
        });
    }

    if let Some(element) = b.node.filter(|&n| document[n].is_element("svg")) {
        let inset = s.border_width;
        let content = Rect::new(
            b.frame.x + s.padding.left + inset,
            b.frame.y + s.padding.top + inset,
            b.frame.w - s.padding.left - s.padding.right - 2.0 * inset,
            b.frame.h - s.padding.top - s.padding.bottom - 2.0 * inset,
        );
        if content.w > 0.0 && content.h > 0.0 {
            items.push(DisplayItem::Svg {
                rect: content,
                source: svg_source(document, element, s.color, content.w, content.h),
            });
        }
        return;
    }

    if !b.lines.is_empty() {
        for fragment in b.lines.iter().flat_map(|line| &line.fragments) {
            match &fragment.content {
                FragmentContent::Text {
                    text,
                    color,
                    font_size,
                    bold,
                } => items.push(DisplayItem::Text {
                    x: fragment.rect.x,
                    baseline: fragment.rect.y + fragment.baseline,
                    text: text.clone(),
                    font_size: *font_size,
                    bold: *bold,
                    color: *color,
                }),
                FragmentContent::Box(index) => add_box(&b.children[*index], document, items),
            }
        }
        return;
    }

    for child in &b.children {
        add_box(child, document, items);
    }
}

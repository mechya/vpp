//! Laying out a whole document.

use vpp_dom::Document;
use vpp_style::{ComputedStyle, StyleSheet, compute_style};

use crate::layouter::Layouter;
use crate::measure::TextMeasure;
use crate::tree::LayoutBox;

/// Lays out `document` in a viewport `viewport_width` CSS pixels wide, with
/// the page's stylesheets in load order. `None` if the document has no root element.
pub fn layout_document(
    document: &Document,
    sheets: &[StyleSheet],
    measure: &dyn TextMeasure,
    viewport_width: f32,
) -> Option<LayoutBox> {
    let html = document[document.root()]
        .children()
        .iter()
        .copied()
        .find(|&n| document.element(n).is_some())?;
    let style = compute_style(document, html, &ComputedStyle::initial(), sheets);
    let layouter = Layouter {
        document,
        sheets,
        measure,
    };
    Some(layouter.lay_out_document(html, style, viewport_width))
}

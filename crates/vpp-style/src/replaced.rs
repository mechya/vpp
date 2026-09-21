//! Replaced elements: those with an intrinsic size instead of laid-out
//! content. Currently only `<svg>`, sized by CSS or its attributes.

use vpp_dom::{Document, NodeId};

use crate::computed::ComputedStyle;
use crate::values::length::Length;
use crate::values::number::parse_number;

/// The size of a replaced element in CSS pixels, or `None` if `element` is
/// not one. A pixel `width` or `height` in CSS wins over the attributes;
/// without either, 16 × 16, as for an icon.
pub fn replaced_size(
    document: &Document,
    element: NodeId,
    style: &ComputedStyle,
) -> Option<(f32, f32)> {
    let data = document.element(element).filter(|e| e.tag == "svg")?;
    let attribute = |name: &str| {
        data.attribute(name)
            .and_then(parse_number)
            .map_or(16.0, |(value, _)| value)
    };
    let width = match style.width {
        Length::Px(px) => px,
        _ => attribute("width"),
    };
    let height = match style.height {
        Length::Px(px) => px,
        _ => attribute("height"),
    };
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_size_from_attributes_or_css() {
        let doc = vpp_dom::parse_html(
            r#"<svg id=a width="24" height="12"></svg><svg id=b></svg><p id=c></p>"#,
        );
        let find = |id| doc.find_by_id(doc.root(), id).unwrap();
        let mut style = ComputedStyle::initial();
        assert_eq!(replaced_size(&doc, find("a"), &style), Some((24.0, 12.0)));
        assert_eq!(replaced_size(&doc, find("b"), &style), Some((16.0, 16.0)));
        style.width = Length::Px(40.0);
        assert_eq!(replaced_size(&doc, find("a"), &style), Some((40.0, 12.0)));
        assert_eq!(replaced_size(&doc, find("c"), &style), None);
    }
}

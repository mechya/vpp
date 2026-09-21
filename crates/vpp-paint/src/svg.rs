//! Inline `<svg>`: the element's subtree becomes a standalone SVG document,
//! drawn by `resvg` (`docs/reference/runtime.md`).
//!
//! As in the C++ engine, shapes with no `fill`, or `fill="currentColor"`,
//! take the element's CSS `color`, so icon sets such as Bootstrap Icons can be
//! coloured with CSS. External files are never loaded: `resvg` is built
//! without image and font support, and has no folder to read from.

use resvg::tiny_skia::{PixmapMut, Transform};
use resvg::usvg;
use vpp_dom::{Document, NodeData, NodeId};
use vpp_layout::Rect;
use vpp_style::Color;

/// SVG names that are camel-case. The HTML parser lower-cases every name,
/// as the C++ engine did, and SVG needs them back.
const CAMEL_CASE: &[&str] = &[
    "viewBox",
    "preserveAspectRatio",
    "linearGradient",
    "radialGradient",
    "gradientUnits",
    "gradientTransform",
    "spreadMethod",
    "clipPath",
    "clipPathUnits",
    "patternUnits",
    "patternContentUnits",
    "patternTransform",
    "maskUnits",
    "maskContentUnits",
    "textPath",
    "stdDeviation",
];

/// A standalone SVG document for `element`, `width` × `height` pixels, with
/// `currentColor` meaning `color`.
pub(crate) fn svg_source(
    document: &Document,
    element: NodeId,
    color: Color,
    width: f32,
    height: f32,
) -> String {
    let mut out = String::new();
    let svg = document
        .element(element)
        .expect("called for an <svg> element");
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    out.push_str(&format!(" width=\"{width}\" height=\"{height}\""));
    out.push_str(&format!(
        " color=\"#{:02x}{:02x}{:02x}\"",
        color.r, color.g, color.b
    ));
    if color.a < 255 {
        out.push_str(&format!(" opacity=\"{}\"", f32::from(color.a) / 255.0));
    }
    if svg.attribute("fill").is_none() {
        out.push_str(" fill=\"currentColor\"");
    }
    for attribute in &svg.attributes {
        if !matches!(
            attribute.name.as_str(),
            "width" | "height" | "xmlns" | "color"
        ) {
            push_attribute(&mut out, &attribute.name, &attribute.value);
        }
    }
    out.push('>');
    for &child in document[element].children() {
        push_node(&mut out, document, child);
    }
    out.push_str("</svg>");
    out
}

fn push_node(out: &mut String, document: &Document, node: NodeId) {
    match document[node].data() {
        NodeData::Text(text) => out.push_str(&escape(text)),
        NodeData::Element(element) => {
            let tag = svg_name(&element.tag);
            out.push('<');
            out.push_str(tag);
            for attribute in &element.attributes {
                push_attribute(out, &attribute.name, &attribute.value);
            }
            out.push('>');
            for &child in document[node].children() {
                push_node(out, document, child);
            }
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        NodeData::Document => {}
    }
}

fn push_attribute(out: &mut String, name: &str, value: &str) {
    // Namespaced attributes lost their prefix in parsing; `href` works without it.
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return;
    }
    out.push(' ');
    out.push_str(svg_name(name));
    out.push_str("=\"");
    out.push_str(&escape(value));
    out.push('"');
}

fn svg_name(lower: &str) -> &str {
    CAMEL_CASE
        .iter()
        .find(|name| name.eq_ignore_ascii_case(lower))
        .copied()
        .unwrap_or(lower)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Draws an SVG document into `rect` (CSS pixels) at `scale`. An SVG that
/// does not parse draws nothing, as a broken image would.
pub(crate) fn draw_svg(pixmap: &mut PixmapMut<'_>, source: &str, rect: Rect, scale: f32) {
    let Ok(tree) = usvg::Tree::from_str(source, &usvg::Options::default()) else {
        return;
    };
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, rect.x * scale, rect.y * scale);
    resvg::render(&tree, transform, pixmap);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(html: &str) -> String {
        let doc = vpp_dom::parse_html(html);
        let svg = doc.find_first(doc.root(), "svg").unwrap();
        svg_source(&doc, svg, Color::rgb(37, 99, 235), 16.0, 16.0)
    }

    #[test]
    fn restores_camel_case_and_uses_the_css_colour() {
        let s = source(r#"<svg viewBox="0 0 16 16" width="16"><path d="M0 0h16v16z"/></svg>"#);
        assert!(s.contains(r#"viewBox="0 0 16 16""#), "{s}");
        assert!(s.contains(r##"color="#2563eb""##), "{s}");
        assert!(s.contains(r#"fill="currentColor""#), "{s}");
        assert!(s.contains(r#"<path d="M0 0h16v16z"></path>"#), "{s}");
        assert!(usvg::Tree::from_str(&s, &usvg::Options::default()).is_ok());
    }

    #[test]
    fn an_explicit_fill_is_kept() {
        let s = source(r#"<svg fill="red"><path d="M0 0h1v1z"/></svg>"#);
        assert!(!s.contains("currentColor"), "{s}");
        assert!(s.contains(r#"fill="red""#), "{s}");
    }

    #[test]
    fn values_are_escaped() {
        let s = source(r#"<svg><title>a &amp; "b" &lt;c&gt;</title></svg>"#);
        assert!(
            s.contains("<title>a &amp; &quot;b&quot; &lt;c&gt;</title>"),
            "{s}"
        );
        assert!(usvg::Tree::from_str(&s, &usvg::Options::default()).is_ok());
    }
}

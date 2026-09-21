//! The cascade: which declarations apply to an element, and in what order.
//!
//! Built-in defaults come first, then the matching rules of the page's
//! stylesheets, then the element's `style=""` attribute. Among declarations,
//! `!important` beats normal, then higher specificity wins, then the later one.
//! `style=""` counts as more specific than any selector.
//! <https://drafts.csswg.org/css-cascade-4/#cascade-sort>

use vpp_dom::{Document, NodeId};

use crate::computed::ComputedStyle;
use crate::parse::parse_declarations;
use crate::properties::apply;
use crate::stylesheet::{Declaration, StyleSheet};
use crate::user_agent::user_agent_style;

/// Specificity given to `style=""` declarations: above any selector.
const INLINE_SPECIFICITY: i32 = 1_000_000;

/// The computed style of element `element`, given its parent's computed
/// style and the page's stylesheets in load order.
pub fn compute_style(
    document: &Document,
    element: NodeId,
    parent: &ComputedStyle,
    sheets: &[StyleSheet],
) -> ComputedStyle {
    let Some(data) = document.element(element) else {
        return parent.inherit();
    };
    let mut style = user_agent_style(&data.tag, parent);

    // (important, specificity, order) for each declaration that applies.
    let mut matched: Vec<(bool, i32, usize, &Declaration)> = Vec::new();
    let mut order = 0;
    for rule in sheets.iter().flat_map(|sheet| &sheet.rules) {
        let best = rule
            .selectors
            .iter()
            .filter(|selector| selector.matches(document, element))
            .map(|selector| selector.specificity())
            .max();
        if let Some(specificity) = best {
            for declaration in &rule.declarations {
                matched.push((declaration.important, specificity, order, declaration));
                order += 1;
            }
        }
    }
    let inline = data
        .attribute("style")
        .map(parse_declarations)
        .unwrap_or_default();
    for declaration in &inline {
        matched.push((
            declaration.important,
            INLINE_SPECIFICITY,
            order,
            declaration,
        ));
        order += 1;
    }
    matched.sort_by_key(|&(important, specificity, order, _)| (important, specificity, order));

    // font-size first, so `em` lengths on the same element resolve against it.
    let (font_size, others): (Vec<_>, Vec<_>) = matched
        .iter()
        .map(|&(_, _, _, declaration)| declaration)
        .partition(|d| d.property == "font-size");
    for declaration in font_size.into_iter().chain(others) {
        apply(declaration, parent, &mut style);
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computed::Display;
    use crate::parse_stylesheet;
    use crate::values::color::Color;
    use crate::values::edges::Edges;
    use crate::values::length::Length;

    /// The computed style of the element with id `t`, computing its ancestors' styles on the way.
    fn style_of(css: &str, html: &str) -> ComputedStyle {
        let doc = vpp_dom::parse_html(html);
        let sheets = [parse_stylesheet(css)];
        let target = doc.find_by_id(doc.root(), "t").unwrap();
        let mut chain: Vec<_> = std::iter::successors(Some(target), |&n| doc[n].parent()).collect();
        chain.reverse();
        chain
            .into_iter()
            .skip(1) // the document root
            .fold(ComputedStyle::initial(), |parent, node| {
                compute_style(&doc, node, &parent, &sheets)
            })
    }

    #[test]
    fn specificity_then_order_then_important() {
        let html = r#"<p id=t class=a>x</p>"#;
        assert_eq!(
            style_of("p.a { color: red } p { color: blue }", html).color,
            Color::rgb(255, 0, 0)
        );
        assert_eq!(
            style_of("p { color: red } p { color: blue }", html).color,
            Color::rgb(0, 0, 255)
        );
        assert_eq!(
            style_of("p { color: red !important } #t.a { color: blue }", html).color,
            Color::rgb(255, 0, 0)
        );
    }

    #[test]
    fn inline_style_beats_rules_but_not_important_rules() {
        let html = r#"<p id=t style="color: green">x</p>"#;
        assert_eq!(
            style_of("#t { color: red }", html).color,
            Color::rgb(0, 128, 0)
        );
        assert_eq!(
            style_of("#t { color: red !important }", html).color,
            Color::rgb(255, 0, 0)
        );
    }

    #[test]
    fn inheritance_and_em() {
        let css = "div { font-size: 20px; color: navy } p { font-size: 1.5em; padding: 1em }";
        let s = style_of(css, r#"<div><p id=t>x</p></div>"#);
        assert_eq!(s.color, Color::rgb(0, 0, 128));
        assert_eq!(s.font_size, 30.0);
        // font-size is applied first, so padding's em uses the element's own size.
        assert_eq!(s.padding, Edges::all(30.0));
    }

    #[test]
    fn user_agent_defaults_can_be_overridden() {
        let s = style_of(
            "button { background: #111; padding: 0 }",
            r#"<button id=t>x</button>"#,
        );
        assert_eq!(s.display, Display::InlineBlock);
        assert_eq!(s.background, Color::rgb(17, 17, 17));
        assert_eq!(s.padding, Edges::default());
    }

    #[test]
    fn margin_auto_and_lengths() {
        let s = style_of(
            ".c { max-width: 400px; margin: 0 auto; width: 50% }",
            r#"<div id=t class=c></div>"#,
        );
        assert!(s.margin_left_auto && s.margin_right_auto);
        assert_eq!(s.max_width, Length::Px(400.0));
        assert_eq!(s.width, Length::Percent(50.0));
    }

    #[test]
    fn borders_flex_and_unknown_properties() {
        let css = ".f { display: flex; flex-direction: column; justify-content: space-between; gap: 8px; \
                   border: solid red; unknown-thing: 5; flex: 2 }";
        let s = style_of(css, r#"<div id=t class=f></div>"#);
        assert_eq!(s.display, Display::Flex);
        assert_eq!(s.border_width, 1.0);
        assert_eq!(s.border_color, Color::rgb(255, 0, 0));
        assert_eq!(s.gap, 8.0);
        assert_eq!(s.flex_grow, 2.0);
    }

    #[test]
    fn invalid_values_leave_the_previous_value() {
        let s = style_of(
            "p { color: blue; color: nonsense; display: sideways }",
            r#"<p id=t>x</p>"#,
        );
        assert_eq!(s.color, Color::rgb(0, 0, 255));
        assert_eq!(s.display, Display::Block);
    }
}

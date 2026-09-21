//! `{{ name }}` substitution for include variables and component properties.
//!
//! In text, a known name is replaced and an unknown one is left as written: it
//! may be a runtime expression later. In attribute values an unknown name is
//! removed, and an attribute that was only a placeholder is dropped, so an
//! unset property leaves a clean element behind.

use std::collections::BTreeMap;

use vpp_dom::{Document, NodeData, NodeId};

/// Names and values available to `{{ }}` placeholders.
pub(crate) type Variables = BTreeMap<String, String>;

/// Substitutes `vars` into every text node and attribute below and including `root`.
pub(crate) fn substitute(document: &mut Document, root: NodeId, vars: &Variables) {
    if vars.is_empty() {
        return;
    }
    let nodes: Vec<_> = document.descendants(root).collect();
    for node in nodes {
        match document[node].data() {
            NodeData::Text(text) => {
                let replaced = substitute_text(text, vars, false);
                document.set_text(node, replaced);
            }
            NodeData::Element(_) => substitute_attributes(document, node, vars),
            NodeData::Document => {}
        }
    }
}

fn substitute_attributes(document: &mut Document, node: NodeId, vars: &Variables) {
    let element = document.element_mut(node).expect("checked by the caller");
    let attributes = element.attributes.clone();
    for attribute in attributes {
        if !attribute.value.contains("{{") {
            continue;
        }
        let value = substitute_text(&attribute.value, vars, true);
        let value = value.trim();
        if value.is_empty() && attribute.value.trim_start().starts_with("{{") {
            element.remove_attribute(&attribute.name);
        } else {
            element.set_attribute(&attribute.name, value);
        }
    }
}

/// Replaces each `{{ name }}` whose name is in `vars`. Unknown names are kept,
/// or removed when `drop_unknown` is set.
pub(crate) fn substitute_text(text: &str, vars: &Variables, drop_unknown: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        let Some(close) = rest[open + 2..].find("}}").map(|n| open + 2 + n) else {
            break;
        };
        out.push_str(&rest[..open]);
        match vars.get(rest[open + 2..close].trim()) {
            Some(value) => out.push_str(value),
            None if drop_unknown => {}
            None => out.push_str(&rest[open..close + 2]),
        }
        rest = &rest[close + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Variables {
        Variables::from([
            ("value".into(), "3".into()),
            ("label".into(), "pages".into()),
        ])
    }

    #[test]
    fn replaces_known_names_and_keeps_unknown_ones_in_text() {
        assert_eq!(
            substitute_text("{{value}} {{ label }} {{ other }}", &vars(), false),
            "3 pages {{ other }}"
        );
        assert_eq!(substitute_text("a {{ value", &vars(), false), "a {{ value");
    }

    #[test]
    fn attributes_drop_unknown_names_and_empty_placeholders() {
        let mut doc = vpp_dom::parse_html(
            r#"<div id="{{ value-id }}" class="stat {{ highlight }}" title="{{label}}"></div>"#,
        );
        let div = doc.find_first(doc.root(), "div").unwrap();
        substitute(&mut doc, div, &vars());
        let element = doc.element(div).unwrap();
        assert_eq!(element.attribute("id"), None);
        assert_eq!(element.attribute("class"), Some("stat"));
        assert_eq!(element.attribute("title"), Some("pages"));
    }

    #[test]
    fn text_nodes_are_substituted_throughout_the_subtree() {
        let mut doc = vpp_dom::parse_html("<p>{{ value }} <b>{{ label }}</b></p>");
        let p = doc.find_first(doc.root(), "p").unwrap();
        substitute(&mut doc, p, &vars());
        assert_eq!(doc.text_content(p), "3 pages");
    }
}

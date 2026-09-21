//! Slots and fills: how a page's content reaches its layout, and a
//! component's children reach its template (`docs/template-syntax.md`).
//!
//! A `<vpp-fill slot="x">` fills `<vpp-slot name="x">`; content outside any
//! fill fills the unnamed slot. A slot's own children are its fallback, used
//! when nothing fills it. `mode="append"` or `mode="prepend"` on a fill adds to
//! the fallback instead of replacing it.

use vpp_dom::{Document, NodeData, NodeId};

/// Content collected from `<vpp-fill>` elements, held in its own document so
/// it can be copied into any slot, any number of times.
pub(crate) struct Fills {
    document: Document,
    fills: Vec<Fill>,
}

struct Fill {
    /// The slot name; empty for the unnamed slot.
    slot: String,
    mode: Mode,
    nodes: Vec<NodeId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Replace,
    Append,
    Prepend,
}

impl Fills {
    /// Moves every child of `container` out of `document` into fills.
    /// Whitespace-only text outside a `<vpp-fill>` is dropped.
    pub(crate) fn take_from(document: &mut Document, container: NodeId) -> Self {
        let mut fills = Fills {
            document: Document::new(),
            fills: Vec::new(),
        };
        let mut unnamed = Fill {
            slot: String::new(),
            mode: Mode::Replace,
            nodes: Vec::new(),
        };
        for child in document.take_children(container) {
            if document[child].is_element("vpp-fill") {
                let element = document.element(child).expect("is an element");
                let slot = element.attribute("slot").unwrap_or_default().to_owned();
                let mode = match element
                    .attribute("mode")
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                {
                    Some("append") => Mode::Append,
                    Some("prepend") => Mode::Prepend,
                    _ => Mode::Replace,
                };
                let nodes = document[child]
                    .children()
                    .iter()
                    .map(|&n| fills.document.import(document, n))
                    .collect();
                fills.fills.push(Fill { slot, mode, nodes });
            } else if !is_whitespace_text(document, child) {
                unnamed.nodes.push(fills.document.import(document, child));
            }
            document.remove(child);
        }
        // Added last, so loose content wins over an explicit unnamed fill.
        if !unnamed.nodes.is_empty() {
            fills.fills.push(unnamed);
        }
        fills
    }

    /// Replaces every slot below `root` with its fill, or with its fallback content.
    pub(crate) fn fill_slots(&self, document: &mut Document, root: NodeId) {
        for slot in find_slots(document, root) {
            let name = slot_name(document, slot).expect("found as a slot");
            let parent = document[slot]
                .parent()
                .expect("a slot below root has a parent");
            let mut index = document.index_in_parent(slot).expect("is a child");
            let fallback = document.take_children(slot);
            document.remove(slot);

            // The last fill for a name wins, as in the C++ expander.
            let replacement = match self.fills.iter().rev().find(|f| f.slot == name) {
                None => fallback,
                Some(fill) => {
                    let content: Vec<_> = fill
                        .nodes
                        .iter()
                        .map(|&n| document.import(&self.document, n))
                        .collect();
                    match fill.mode {
                        Mode::Append => fallback.into_iter().chain(content).collect(),
                        Mode::Prepend => content.into_iter().chain(fallback).collect(),
                        Mode::Replace => {
                            for node in fallback {
                                document.remove(node);
                            }
                            content
                        }
                    }
                }
            };
            for node in replacement {
                document.insert_child(parent, index, node);
                index += 1;
            }
        }
    }
}

/// The slot's name if `node` is a slot: `<vpp-slot name>`, or the
/// `<meta name="vpp-slot" content>` placeholder used inside `<head>`.
fn slot_name(document: &Document, node: NodeId) -> Option<String> {
    let element = document.element(node)?;
    match element.tag.as_str() {
        "vpp-slot" => Some(element.attribute("name").unwrap_or_default().to_owned()),
        "meta" if element.attribute("name") == Some("vpp-slot") => {
            Some(element.attribute("content").unwrap_or_default().to_owned())
        }
        _ => None,
    }
}

/// Slots below `root` in document order, not looking inside slots.
fn find_slots(document: &Document, root: NodeId) -> Vec<NodeId> {
    let mut slots = Vec::new();
    let mut pending: Vec<NodeId> = document[root].children().iter().rev().copied().collect();
    while let Some(node) = pending.pop() {
        if slot_name(document, node).is_some() {
            slots.push(node);
        } else {
            pending.extend(document[node].children().iter().rev().copied());
        }
    }
    slots
}

fn is_whitespace_text(document: &Document, node: NodeId) -> bool {
    matches!(document[node].data(), NodeData::Text(t) if t.chars().all(|c| c.is_ascii_whitespace()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fills taken from `source`'s body, applied to `target`; returns the result's body text.
    fn apply(source: &str, target: &str) -> String {
        let mut page = vpp_dom::parse_html(&crate::preprocess_template_html(source));
        let body = page.find_first(page.root(), "body").unwrap();
        let fills = Fills::take_from(&mut page, body);

        let mut layout = vpp_dom::parse_html(&crate::preprocess_template_html(target));
        let root = layout.root();
        fills.fill_slots(&mut layout, root);
        let body = layout.find_first(layout.root(), "body").unwrap();
        layout.text_content(body)
    }

    #[test]
    fn named_and_unnamed_fills() {
        let text = apply(
            r#"<vpp-fill slot="a">A</vpp-fill>main"#,
            r#"[<vpp-slot name="a" />] [<vpp-slot />]"#,
        );
        assert_eq!(text, "[A] [main]");
    }

    #[test]
    fn unfilled_slot_keeps_its_fallback() {
        assert_eq!(
            apply("", r#"[<vpp-slot name="a">fallback</vpp-slot>]"#),
            "[fallback]"
        );
    }

    #[test]
    fn append_and_prepend_keep_the_fallback() {
        let target = r#"[<vpp-slot name="s">F</vpp-slot>]"#;
        assert_eq!(
            apply(r#"<vpp-fill slot="s" mode="append">X</vpp-fill>"#, target),
            "[FX]"
        );
        assert_eq!(
            apply(r#"<vpp-fill slot="s" mode="PREPEND">X</vpp-fill>"#, target),
            "[XF]"
        );
        assert_eq!(apply(r#"<vpp-fill slot="s">X</vpp-fill>"#, target), "[X]");
    }

    #[test]
    fn one_fill_serves_several_slots_of_the_same_name() {
        assert_eq!(
            apply(
                r#"<vpp-fill slot="s">X</vpp-fill>"#,
                r#"<vpp-slot name="s" />-<vpp-slot name="s" />"#
            ),
            "X-X"
        );
    }

    #[test]
    fn slots_in_head_are_filled_in_place() {
        let mut page = vpp_dom::parse_html(r#"<vpp-fill slot="head"><title>T</title></vpp-fill>"#);
        let body = page.find_first(page.root(), "body").unwrap();
        let fills = Fills::take_from(&mut page, body);
        let mut layout = vpp_dom::parse_html(&crate::preprocess_template_html(
            r#"<head><vpp-slot name="head" /><link rel="x"></head><body></body>"#,
        ));
        let root = layout.root();
        fills.fill_slots(&mut layout, root);
        let head = layout.find_first(layout.root(), "head").unwrap();
        let tags: Vec<_> = layout[head]
            .children()
            .iter()
            .filter_map(|&c| layout.element(c).map(|e| e.tag.clone()))
            .collect();
        assert_eq!(tags, ["title", "link"]);
    }
}

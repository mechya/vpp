//! HTML text to a [`Document`], with `html5ever` doing the HTML5 tokenising and
//! tree construction.
//!
//! `html5ever` builds the tree by calling a [`TreeSink`]; this one writes
//! straight into VPP's arena. Like the C++ version, it keeps elements and text
//! only: comments, doctypes, processing instructions, and the contents of
//! `<template>` elements are dropped. Tag and attribute names are lower-cased,
//! and attributes keep only their local name (`xlink:href` becomes `href`).

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use html5ever::interface::{ElemName, ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{Attribute as HtmlAttribute, LocalName, Namespace, QualName, parse_document};

use crate::document::Document;
use crate::node::{Element, NodeData, NodeId};

/// Parses an HTML document. Never fails: HTML parsing recovers from any input,
/// as browsers do.
pub fn parse_html(source: &str) -> Document {
    parse_document(Sink::default(), Default::default()).one(source)
}

#[derive(Default)]
struct Sink {
    document: RefCell<Document>,
    /// The full name of each element, which the tree builder asks for; the DOM
    /// keeps only the lower-cased local name.
    names: RefCell<HashMap<NodeId, QualName>>,
    /// Comments and processing instructions: created because the tree builder
    /// needs a handle, never attached.
    dropped: RefCell<HashSet<NodeId>>,
    /// `<template>` element to its contents, which stay detached.
    templates: RefCell<HashMap<NodeId, NodeId>>,
}

/// An element's name, owned so it can be returned out of the `RefCell`.
#[derive(Debug)]
struct OwnedName(QualName);

impl ElemName for OwnedName {
    fn ns(&self) -> &Namespace {
        &self.0.ns
    }

    fn local_name(&self) -> &LocalName {
        &self.0.local
    }
}

impl Sink {
    /// A node the tree builder will not see attached: a comment or instruction.
    fn dropped_node(&self) -> NodeId {
        let id = self.document.borrow_mut().create(NodeData::Document);
        self.dropped.borrow_mut().insert(id);
        id
    }

    /// Appends `child` at the end of `parent`, merging text with a text node already there.
    fn append_to(&self, parent: NodeId, child: NodeOrText<NodeId>) {
        let mut doc = self.document.borrow_mut();
        match child {
            NodeOrText::AppendNode(node) => {
                if !self.dropped.borrow().contains(&node) {
                    doc.detach(node);
                    doc.append_child(parent, node);
                }
            }
            NodeOrText::AppendText(text) => {
                if let Some(&last) = doc[parent].children.last() {
                    if let NodeData::Text(existing) = &mut doc[last].data {
                        existing.push_str(&text);
                        return;
                    }
                }
                let node = doc.create_text(text.to_string());
                doc.append_child(parent, node);
            }
        }
    }
}

impl TreeSink for Sink {
    type Handle = NodeId;
    type Output = Document;
    type ElemName<'a> = OwnedName;

    fn finish(self) -> Document {
        let mut document = self.document.into_inner();
        for id in self.dropped.into_inner() {
            document.remove(id);
        }
        for (_, contents) in self.templates.into_inner() {
            document.remove(contents);
        }
        document
    }

    // Parse errors are recovered from, as the HTML standard requires; VPP does not report them.
    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> NodeId {
        self.document.borrow().root()
    }

    fn elem_name<'a>(&'a self, target: &'a NodeId) -> OwnedName {
        OwnedName(self.names.borrow()[target].clone())
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<HtmlAttribute>,
        flags: ElementFlags,
    ) -> NodeId {
        let mut element = Element::new(&name.local);
        for attr in attrs {
            element.set_attribute(&attr.name.local, attr.value.to_string());
        }
        let id = self
            .document
            .borrow_mut()
            .create(NodeData::Element(element));
        self.names.borrow_mut().insert(id, name);
        if flags.template {
            let contents = self.document.borrow_mut().create(NodeData::Document);
            self.templates.borrow_mut().insert(id, contents);
        }
        id
    }

    fn create_comment(&self, _text: StrTendril) -> NodeId {
        self.dropped_node()
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> NodeId {
        self.dropped_node()
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        self.append_to(*parent, child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        let has_parent = self.document.borrow()[*element].parent.is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append_to(*prev_element, child);
        }
    }

    fn append_doctype_to_document(&self, _: StrTendril, _: StrTendril, _: StrTendril) {}

    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        self.templates.borrow()[target]
    }

    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &NodeId, new_node: NodeOrText<NodeId>) {
        let mut doc = self.document.borrow_mut();
        let Some(parent) = doc[*sibling].parent else {
            return;
        };
        let index = doc
            .index_in_parent(*sibling)
            .expect("a child is in its parent");
        match new_node {
            NodeOrText::AppendNode(node) => {
                if !self.dropped.borrow().contains(&node) {
                    doc.detach(node);
                    let index = doc.index_in_parent(*sibling).unwrap_or(index);
                    doc.insert_child(parent, index, node);
                }
            }
            NodeOrText::AppendText(text) => {
                if index > 0 {
                    let before = doc[parent].children[index - 1];
                    if let NodeData::Text(existing) = &mut doc[before].data {
                        existing.push_str(&text);
                        return;
                    }
                }
                let node = doc.create_text(text.to_string());
                doc.insert_child(parent, index, node);
            }
        }
    }

    fn add_attrs_if_missing(&self, target: &NodeId, attrs: Vec<HtmlAttribute>) {
        let mut doc = self.document.borrow_mut();
        if let Some(element) = doc.element_mut(*target) {
            for attr in attrs {
                if element
                    .attribute(&attr.name.local.to_ascii_lowercase())
                    .is_none()
                {
                    element.set_attribute(&attr.name.local, attr.value.to_string());
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &NodeId) {
        self.document.borrow_mut().detach(*target);
    }

    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        let mut doc = self.document.borrow_mut();
        for child in doc.take_children(*node) {
            doc.append_child(*new_parent, child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A compact view of the tree for comparisons: `tag[attr=value](children)` and `"text"`.
    fn outline(doc: &Document, id: NodeId) -> String {
        let children: Vec<_> = doc[id]
            .children()
            .iter()
            .map(|&c| outline(doc, c))
            .collect();
        match doc[id].data() {
            NodeData::Document => children.join(""),
            NodeData::Text(text) => format!("{text:?}"),
            NodeData::Element(e) => {
                let attrs: String = e
                    .attributes
                    .iter()
                    .map(|a| format!("[{}={}]", a.name, a.value))
                    .collect();
                format!("{}{attrs}({})", e.tag, children.join(","))
            }
        }
    }

    #[test]
    fn builds_the_standard_document_structure() {
        let doc = parse_html("<p>hi");
        assert_eq!(outline(&doc, doc.root()), r#"html(head(),body(p("hi")))"#);
    }

    #[test]
    fn keeps_attributes_in_order() {
        let doc = parse_html(r#"<a HREF="x" class=c id=i>t</a>"#);
        let a = doc.find_first(doc.root(), "a").unwrap();
        let e = doc.element(a).unwrap();
        let names: Vec<_> = e.attributes.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["href", "class", "id"]);
    }

    #[test]
    fn drops_comments_doctype_and_template_contents() {
        let doc =
            parse_html("<!DOCTYPE html><!-- c --><p>a<!-- c -->b</p><template><i>t</i></template>");
        // With the comment dropped, "a" and "b" arrive as adjacent text and are merged.
        assert_eq!(
            outline(&doc, doc.root()),
            r#"html(head(),body(p("ab"),template()))"#
        );
        // Nothing dropped is left in the arena.
        assert_eq!(doc.nodes.len(), doc.descendants(doc.root()).count());
    }

    #[test]
    fn recovers_from_broken_html_like_a_browser() {
        // Misnested tags are fixed by the HTML5 adoption agency algorithm.
        let doc = parse_html("<b>1<i>2</b>3</i>");
        assert_eq!(
            outline(&doc, doc.root()),
            r#"html(head(),body(b("1",i("2")),i("3")))"#
        );
        // Table content in the wrong place is moved before the table ("foster parenting").
        let doc = parse_html("<table>x<tr><td>y</td></tr></table>");
        let body = doc.find_first(doc.root(), "body").unwrap();
        assert_eq!(doc[doc[body].children()[0]].as_text(), Some("x"));
    }

    #[test]
    fn svg_names_are_lower_cased_like_the_cpp_version() {
        let doc = parse_html(r#"<svg viewBox="0 0 16 16"><linearGradient/></svg>"#);
        let svg = doc.find_first(doc.root(), "svg").unwrap();
        assert_eq!(
            doc.element(svg).unwrap().attribute("viewbox"),
            Some("0 0 16 16")
        );
        assert!(doc.find_first(svg, "lineargradient").is_some());
    }

    #[test]
    fn style_and_script_keep_their_raw_text() {
        let doc = parse_html("<style>a > b { }</style><script>if (a < b) {}</script>");
        let style = doc.find_first(doc.root(), "style").unwrap();
        let script = doc.find_first(doc.root(), "script").unwrap();
        assert_eq!(doc.raw_text(style), "a > b { }");
        assert_eq!(doc.raw_text(script), "if (a < b) {}");
    }
}

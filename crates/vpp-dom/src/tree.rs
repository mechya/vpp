//! Editing the tree: every operation that changes parent and child links.
//!
//! Keeping them in one file keeps the links consistent: a node has at most one
//! parent, and appears in that parent's children exactly once.

use crate::document::Document;
use crate::node::NodeId;

impl Document {
    /// Appends detached node `child` as the last child of `parent`.
    ///
    /// # Panics
    ///
    /// If `child` already has a parent; [`Document::detach`] it first.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        let len = self[parent].children.len();
        self.insert_child(parent, len, child);
    }

    /// Inserts detached node `child` at `index` among `parent`'s children, or
    /// last if `index` is past the end.
    ///
    /// # Panics
    ///
    /// If `child` already has a parent, or is `parent` itself.
    pub fn insert_child(&mut self, parent: NodeId, index: usize, child: NodeId) {
        assert!(
            self[child].parent.is_none(),
            "insert_child: the node already has a parent"
        );
        assert!(
            parent != child,
            "insert_child: a node cannot contain itself"
        );
        self[child].parent = Some(parent);
        let children = &mut self[parent].children;
        let index = index.min(children.len());
        children.insert(index, child);
    }

    /// Detaches `id` from its parent. It stays in the document, with its own
    /// subtree, until it is inserted again or removed.
    pub fn detach(&mut self, id: NodeId) {
        if let Some(parent) = self[id].parent.take() {
            self[parent].children.retain(|&c| c != id);
        }
    }

    /// Detaches and returns all children of `parent`, in order.
    pub fn take_children(&mut self, parent: NodeId) -> Vec<NodeId> {
        let children = std::mem::take(&mut self[parent].children);
        for &child in &children {
            self[child].parent = None;
        }
        children
    }

    /// The position of `child` among its parent's children.
    pub fn index_in_parent(&self, child: NodeId) -> Option<usize> {
        let parent = self[child].parent?;
        self[parent].children.iter().position(|&c| c == child)
    }

    /// Detaches `id` and removes it and its whole subtree from the document.
    /// Their ids become stale.
    pub fn remove(&mut self, id: NodeId) {
        self.detach(id);
        let mut pending = vec![id];
        while let Some(next) = pending.pop() {
            if let Some(node) = self.nodes.remove(next) {
                pending.extend(node.children);
            }
        }
    }

    /// Copies the subtree at `source_id` in another document into this one,
    /// and returns the detached copy.
    pub fn import(&mut self, source: &Document, source_id: NodeId) -> NodeId {
        let nodes: Vec<_> = source
            .descendants(source_id)
            .map(|n| (n, source[n].parent, source[n].data.clone()))
            .collect();
        self.copy_in_order(source_id, nodes)
    }

    /// A detached deep copy of `id` within this document.
    pub fn clone_subtree(&mut self, id: NodeId) -> NodeId {
        let nodes: Vec<_> = self
            .descendants(id)
            .map(|n| (n, self[n].parent, self[n].data.clone()))
            .collect();
        self.copy_in_order(id, nodes)
    }

    /// Recreates `nodes`, given in document order with their original parents,
    /// and returns the copy of `top`. Iterative, so deep trees cannot overflow the stack.
    fn copy_in_order(
        &mut self,
        top: NodeId,
        nodes: Vec<(NodeId, Option<NodeId>, crate::node::NodeData)>,
    ) -> NodeId {
        let mut copies = std::collections::HashMap::with_capacity(nodes.len());
        for (original, parent, data) in nodes {
            let copy = self.create(data);
            if original != top {
                // Document order: the parent was copied before its children.
                let parent_copy = copies[&parent.expect("below top, so it has a parent")];
                self.append_child(parent_copy, copy);
            }
            copies.insert(original, copy);
        }
        copies[&top]
    }

    /// `id` and every node below it, in document order (depth first).
    pub fn descendants(&self, id: NodeId) -> Descendants<'_> {
        Descendants {
            document: self,
            stack: vec![id],
        }
    }
}

/// Iterator over a subtree in document order; see [`Document::descendants`].
pub struct Descendants<'a> {
    document: &'a Document,
    stack: Vec<NodeId>,
}

impl Iterator for Descendants<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let id = self.stack.pop()?;
        self.stack
            .extend(self.document[id].children.iter().rev().copied());
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// root > div > (p, "text")
    fn sample() -> (Document, NodeId, NodeId, NodeId) {
        let mut doc = Document::new();
        let div = doc.create_element("div");
        let p = doc.create_element("p");
        let text = doc.create_text("text");
        doc.append_child(doc.root(), div);
        doc.append_child(div, p);
        doc.append_child(div, text);
        (doc, div, p, text)
    }

    #[test]
    fn append_and_insert_keep_links_consistent() {
        let (mut doc, div, p, text) = sample();
        let first = doc.create_element("h1");
        doc.insert_child(div, 0, first);
        assert_eq!(doc[div].children(), &[first, p, text]);
        assert_eq!(doc[first].parent(), Some(div));
        assert_eq!(doc.index_in_parent(text), Some(2));

        let last = doc.create_element("footer");
        doc.insert_child(div, 99, last);
        assert_eq!(doc[div].children().last(), Some(&last));
    }

    #[test]
    fn detach_keeps_the_subtree() {
        let (mut doc, div, p, _) = sample();
        doc.detach(div);
        assert!(doc[doc.root()].children().is_empty());
        assert_eq!(doc[div].parent(), None);
        assert_eq!(doc[div].children()[0], p);
    }

    #[test]
    fn remove_makes_the_whole_subtree_stale() {
        let (mut doc, div, p, text) = sample();
        doc.remove(div);
        assert!(!doc.contains(div));
        assert!(!doc.contains(p));
        assert!(!doc.contains(text));
        assert!(doc.get(p).is_none());
        // A new node never reuses a stale id.
        let fresh = doc.create_element("p");
        assert_ne!(fresh, p);
        assert!(doc.get(p).is_none());
    }

    #[test]
    fn take_children_detaches_them_in_order() {
        let (mut doc, div, p, text) = sample();
        assert_eq!(doc.take_children(div), vec![p, text]);
        assert!(doc[div].children().is_empty());
        assert_eq!(doc[p].parent(), None);
    }

    #[test]
    fn clone_subtree_is_deep_and_detached() {
        let (mut doc, div, _, _) = sample();
        let copy = doc.clone_subtree(div);
        assert_ne!(copy, div);
        assert_eq!(doc[copy].parent(), None);
        assert_eq!(doc[copy].children().len(), 2);
        assert_eq!(doc[doc[copy].children()[1]].as_text(), Some("text"));
    }

    #[test]
    fn descendants_are_in_document_order() {
        let (doc, div, p, text) = sample();
        let order: Vec<_> = doc.descendants(doc.root()).collect();
        assert_eq!(order, vec![doc.root(), div, p, text]);
    }

    #[test]
    #[should_panic(expected = "already has a parent")]
    fn a_node_cannot_have_two_parents() {
        let (mut doc, _, p, _) = sample();
        let other = doc.create_element("section");
        doc.append_child(other, p);
    }
}

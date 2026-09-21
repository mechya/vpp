//! A document: all of its nodes in one arena, and the root.

use std::ops::{Index, IndexMut};

use slotmap::SlotMap;

use crate::node::{Element, Node, NodeData, NodeId};

/// A document tree. Nodes live in the document and refer to each other by
/// [`NodeId`]; editing the tree goes through the methods in `tree.rs`.
///
/// Indexing with a removed node's id panics. Use [`Document::get`] where an id
/// may be stale.
#[derive(Debug, Clone)]
pub struct Document {
    pub(crate) nodes: SlotMap<NodeId, Node>,
    root: NodeId,
}

impl Document {
    /// An empty document: only the root.
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(Node::new(NodeData::Document));
        Self { nodes, root }
    }

    /// The root node. Its data is [`NodeData::Document`].
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// The node, or `None` if it was removed.
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Whether `id` still refers to a node of this document.
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    /// A new detached element. The tag name is lower-cased.
    pub fn create_element(&mut self, tag: &str) -> NodeId {
        self.nodes
            .insert(Node::new(NodeData::Element(Element::new(tag))))
    }

    /// A new detached text node.
    pub fn create_text(&mut self, text: impl Into<String>) -> NodeId {
        self.nodes.insert(Node::new(NodeData::Text(text.into())))
    }

    /// A new detached node with the given data.
    pub(crate) fn create(&mut self, data: NodeData) -> NodeId {
        self.nodes.insert(Node::new(data))
    }

    /// The element data of `id`, if it is an element.
    pub fn element(&self, id: NodeId) -> Option<&Element> {
        self[id].as_element()
    }

    /// Mutable element data of `id`, if it is an element.
    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut Element> {
        match &mut self[id].data {
            NodeData::Element(element) => Some(element),
            _ => None,
        }
    }

    /// Replaces the character data of text node `id`. Does nothing to other nodes.
    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) {
        if let NodeData::Text(existing) = &mut self[id].data {
            *existing = text.into();
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<NodeId> for Document {
    type Output = Node;

    fn index(&self, id: NodeId) -> &Node {
        self.nodes
            .get(id)
            .expect("NodeId refers to a node that was removed")
    }
}

impl IndexMut<NodeId> for Document {
    fn index_mut(&mut self, id: NodeId) -> &mut Node {
        self.nodes
            .get_mut(id)
            .expect("NodeId refers to a node that was removed")
    }
}

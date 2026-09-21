//! The node types of the document tree.

use slotmap::new_key_type;

new_key_type! {
    /// A handle to a node in one [`crate::Document`].
    ///
    /// Ids are generational: once a node is removed, its id never finds another
    /// node, so a stale handle (for example one kept by a script) is detected
    /// instead of silently reaching the wrong element.
    pub struct NodeId;
}

/// One node: its data and its place in the tree.
#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) data: NodeData,
}

impl Node {
    pub(crate) fn new(data: NodeData) -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            data,
        }
    }

    /// The parent, or `None` for the document root and detached nodes.
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// The children, in document order.
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// What kind of node this is, and its contents.
    pub fn data(&self) -> &NodeData {
        &self.data
    }

    /// The element data, if this is an element.
    pub fn as_element(&self) -> Option<&Element> {
        match &self.data {
            NodeData::Element(element) => Some(element),
            _ => None,
        }
    }

    /// The character data, if this is a text node.
    pub fn as_text(&self) -> Option<&str> {
        match &self.data {
            NodeData::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Whether this is an element with the given lower-case tag name.
    pub fn is_element(&self, tag: &str) -> bool {
        self.as_element().is_some_and(|e| e.tag == tag)
    }
}

/// What a node holds. Comments, doctypes, and processing instructions are not
/// kept: nothing in VPP renders them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeData {
    /// The root of a document.
    Document,
    /// An element.
    Element(Element),
    /// Character data, exactly as in the source.
    Text(String),
}

/// An element's tag name and attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The tag name, in lower case.
    pub tag: String,
    /// The attributes in source order, names in lower case, each name at most once.
    pub attributes: Vec<Attribute>,
}

impl Element {
    /// An element with no attributes. The tag name is lower-cased.
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_ascii_lowercase(),
            attributes: Vec::new(),
        }
    }

    /// The value of attribute `name`, if present.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Sets attribute `name` (lower-cased), replacing its value if it exists.
    pub fn set_attribute(&mut self, name: &str, value: impl Into<String>) {
        let name = name.to_ascii_lowercase();
        let value = value.into();
        match self.attributes.iter_mut().find(|a| a.name == name) {
            Some(existing) => existing.value = value,
            None => self.attributes.push(Attribute { name, value }),
        }
    }

    /// Removes attribute `name`, if present.
    pub fn remove_attribute(&mut self, name: &str) {
        self.attributes.retain(|a| a.name != name);
    }

    /// The `id` attribute, if present.
    pub fn id(&self) -> Option<&str> {
        self.attribute("id")
    }
}

/// One attribute of an element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The name, in lower case.
    pub name: String,
    /// The value.
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_and_attribute_names_are_lower_case() {
        let mut e = Element::new("DIV");
        e.set_attribute("Data-X", "1");
        assert_eq!(e.tag, "div");
        assert_eq!(e.attribute("data-x"), Some("1"));
    }

    #[test]
    fn setting_an_attribute_twice_replaces_it() {
        let mut e = Element::new("a");
        e.set_attribute("href", "one");
        e.set_attribute("href", "two");
        assert_eq!(e.attributes.len(), 1);
        assert_eq!(e.attribute("href"), Some("two"));
        e.remove_attribute("href");
        assert_eq!(e.attribute("href"), None);
    }
}

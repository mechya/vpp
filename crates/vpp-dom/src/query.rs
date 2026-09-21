//! Finding nodes and reading text.

use crate::document::Document;
use crate::node::NodeId;

impl Document {
    /// The first element in `id`'s subtree, including `id`, whose `id` attribute is `wanted`.
    pub fn find_by_id(&self, id: NodeId, wanted: &str) -> Option<NodeId> {
        self.descendants(id)
            .find(|&n| self[n].as_element().and_then(|e| e.id()) == Some(wanted))
    }

    /// The first element in `id`'s subtree, including `id`, with lower-case tag name `tag`.
    pub fn find_first(&self, id: NodeId, tag: &str) -> Option<NodeId> {
        self.descendants(id).find(|&n| self[n].is_element(tag))
    }

    /// The nearest of `id` and its ancestors that is an element with tag name `tag`.
    pub fn closest(&self, id: NodeId, tag: &str) -> Option<NodeId> {
        std::iter::successors(Some(id), |&n| self[n].parent).find(|&n| self[n].is_element(tag))
    }

    /// The text of every text node below `id`, with each run of whitespace
    /// collapsed to one space and none at the start or end.
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        let mut pending_space = false;
        for n in self.descendants(id) {
            for c in self[n].as_text().unwrap_or_default().chars() {
                if c.is_ascii_whitespace() {
                    pending_space = true;
                } else {
                    if pending_space && !out.is_empty() {
                        out.push(' ');
                    }
                    pending_space = false;
                    out.push(c);
                }
            }
        }
        out
    }

    /// The text of the text nodes directly inside `id`, unchanged: the source of a `<style>` or `<script>`.
    pub fn raw_text(&self, id: NodeId) -> String {
        self[id]
            .children
            .iter()
            .filter_map(|&c| self[c].as_text())
            .collect()
    }

    /// Replaces all of `id`'s children with one text node.
    pub fn set_text_content(&mut self, id: NodeId, text: impl Into<String>) {
        for child in self.take_children(id) {
            self.remove(child);
        }
        let text = self.create_text(text);
        self.append_child(id, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Document {
        crate::parse_html(
            "<main id=m><p class=x>Hello,\n   <b>big</b>  world </p><p id=two>2</p></main>",
        )
    }

    #[test]
    fn finds_by_id_and_tag() {
        let doc = sample();
        let two = doc.find_by_id(doc.root(), "two").unwrap();
        assert_eq!(doc.text_content(two), "2");
        let b = doc.find_first(doc.root(), "b").unwrap();
        assert_eq!(doc.text_content(b), "big");
        assert_eq!(doc.find_by_id(doc.root(), "none"), None);
    }

    #[test]
    fn closest_includes_the_node_itself() {
        let doc = sample();
        let b = doc.find_first(doc.root(), "b").unwrap();
        let main = doc.find_by_id(doc.root(), "m").unwrap();
        assert_eq!(doc.closest(b, "main"), Some(main));
        assert_eq!(doc.closest(b, "b"), Some(b));
        assert_eq!(doc.closest(b, "table"), None);
    }

    #[test]
    fn text_content_collapses_whitespace() {
        let doc = sample();
        let p = doc.find_first(doc.root(), "p").unwrap();
        assert_eq!(doc.text_content(p), "Hello, big world");
    }

    #[test]
    fn set_text_content_replaces_children() {
        let mut doc = sample();
        let p = doc.find_first(doc.root(), "p").unwrap();
        let b = doc.find_first(p, "b").unwrap();
        doc.set_text_content(p, "new");
        assert_eq!(doc.text_content(p), "new");
        assert!(!doc.contains(b));
    }
}

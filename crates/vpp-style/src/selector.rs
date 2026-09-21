//! Selectors: the supported subset, their specificity, and matching them against elements.
//!
//! VPP supports type (`p`), universal (`*`), class (`.card`), and id (`#go`)
//! selectors, compounds of them (`button.primary#go`), and the descendant
//! (`.card p`) and child (`ul > li`) combinators
//! (`docs/reference/runtime.md`, "Supported CSS"). Pseudo-classes, attribute
//! selectors, and the sibling combinators are not supported yet.

use vpp_dom::{Document, NodeId};

/// A selector with combinators, such as `.card > p b`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// The compound selectors, leftmost first. Never empty.
    pub compounds: Vec<CompoundSelector>,
    /// `combinators[i]` joins `compounds[i]` and `compounds[i + 1]`, so there is one fewer than compounds.
    pub combinators: Vec<Combinator>,
}

/// One compound selector, such as `button.primary#go` or `*`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompoundSelector {
    /// The lower-case tag name, or `None` for any element.
    pub tag: Option<String>,
    /// The required id.
    pub id: Option<String>,
    /// The required classes.
    pub classes: Vec<String>,
}

/// How two compound selectors are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace: the left side matches any ancestor.
    Descendant,
    /// `>`: the left side matches the parent.
    Child,
}

impl Selector {
    /// The selector's specificity, as one number that orders like the
    /// (ids, classes, types) triple for up to 99 of each:
    /// `ids * 10000 + classes * 100 + types`.
    ///
    /// <https://drafts.csswg.org/selectors-4/#specificity-rules>
    pub fn specificity(&self) -> i32 {
        let (mut ids, mut classes, mut types) = (0i32, 0i32, 0i32);
        for compound in &self.compounds {
            ids += i32::from(compound.id.is_some());
            classes += compound.classes.len() as i32;
            types += i32::from(compound.tag.is_some());
        }
        ids.saturating_mul(10000)
            .saturating_add(classes.saturating_mul(100))
            .saturating_add(types)
    }

    /// Whether `element` in `document` matches this selector.
    pub fn matches(&self, document: &Document, element: NodeId) -> bool {
        !self.compounds.is_empty() && self.matches_from(self.compounds.len() - 1, document, element)
    }

    /// Whether `element` matches compounds `0..=index`, with compound `index` matching `element` itself.
    fn matches_from(&self, index: usize, document: &Document, element: NodeId) -> bool {
        if !self.compounds[index].matches(document, element) {
            return false;
        }
        if index == 0 {
            return true;
        }
        let parent = document[element].parent();
        match self.combinators[index - 1] {
            Combinator::Child => parent.is_some_and(|p| self.matches_from(index - 1, document, p)),
            Combinator::Descendant => std::iter::successors(parent, |&n| document[n].parent())
                .any(|a| self.matches_from(index - 1, document, a)),
        }
    }
}

impl CompoundSelector {
    /// Whether `node` is an element satisfying every part of this compound.
    pub fn matches(&self, document: &Document, node: NodeId) -> bool {
        let Some(element) = document.element(node) else {
            return false;
        };
        if self.tag.as_ref().is_some_and(|tag| *tag != element.tag) {
            return false;
        }
        if self
            .id
            .as_deref()
            .is_some_and(|id| element.id() != Some(id))
        {
            return false;
        }
        let classes = element.attribute("class").unwrap_or_default();
        self.classes
            .iter()
            .all(|wanted| classes.split_ascii_whitespace().any(|c| c == wanted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_stylesheet;

    fn selector(text: &str) -> Selector {
        let sheet = parse_stylesheet(&format!("{text} {{ color: red }}"));
        sheet.rules[0].selectors[0].clone()
    }

    fn matches(selector_text: &str, html: &str, id: &str) -> bool {
        let doc = vpp_dom::parse_html(html);
        let element = doc.find_by_id(doc.root(), id).unwrap();
        selector(selector_text).matches(&doc, element)
    }

    #[test]
    fn specificity_counts_ids_classes_and_types() {
        assert_eq!(selector("*").specificity(), 0);
        assert_eq!(selector("p").specificity(), 1);
        assert_eq!(selector(".a.b").specificity(), 200);
        assert_eq!(selector("#x div.c > p").specificity(), 10102);
    }

    #[test]
    fn compound_needs_every_part() {
        let html = r#"<button id=t class="primary big">x</button>"#;
        assert!(matches("button.primary", html, "t"));
        assert!(matches(".big.primary#t", html, "t"));
        assert!(!matches("button.secondary", html, "t"));
        assert!(!matches("a.primary", html, "t"));
    }

    #[test]
    fn descendant_matches_any_ancestor_child_only_the_parent() {
        let html = r#"<div class=card><section><p id=t>x</p></section></div>"#;
        assert!(matches(".card p", html, "t"));
        assert!(matches("section > p", html, "t"));
        assert!(!matches(".card > p", html, "t"));
        assert!(matches(".card > section > p", html, "t"));
        assert!(matches("div section p", html, "t"));
        assert!(!matches("p p", html, "t"));
    }

    #[test]
    fn class_match_is_by_whole_word() {
        assert!(!matches(".prim", r#"<p id=t class="primary">x</p>"#, "t"));
    }
}

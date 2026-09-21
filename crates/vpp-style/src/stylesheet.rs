//! A parsed stylesheet: rules of selectors and declarations.

use crate::selector::Selector;

/// A parsed stylesheet, in source order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleSheet {
    /// The rules, in source order. Order matters: among equal specificity, the later rule wins.
    pub rules: Vec<Rule>,
}

impl StyleSheet {
    /// Adds `other`'s rules after this sheet's, as if the two were one file.
    pub fn append(&mut self, other: StyleSheet) {
        self.rules.extend(other.rules);
    }

    /// Whether the sheet has no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// One style rule: `selectors { declarations }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The selectors it applies to; never empty.
    pub selectors: Vec<Selector>,
    /// Its declarations, in source order; never empty.
    pub declarations: Vec<Declaration>,
}

/// One declaration: `property: value` or `property: value !important`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The property name, lower-cased, except custom properties (`--name`), which are case-sensitive.
    pub property: String,
    /// The value as written, trimmed, with comments replaced by a space. Interpreted later, when computing styles.
    pub value: String,
    /// Whether it was marked `!important`.
    pub important: bool,
}

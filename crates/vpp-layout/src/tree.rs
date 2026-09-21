//! The layout tree: positioned boxes and lines of text, ready to paint.
//!
//! It is separate from the DOM. A box may have no element (the anonymous box
//! around a run of inline content), and an element with `display: none` has
//! no box.

use vpp_dom::NodeId;
use vpp_style::{Color, ComputedStyle};

use crate::geometry::Rect;

/// One box: an element's border box, or an anonymous box holding lines.
#[derive(Debug, Clone)]
pub struct LayoutBox {
    /// The element, or `None` for an anonymous box.
    pub node: Option<NodeId>,
    /// The element's computed style (an anonymous box inherits its parent's text properties).
    pub style: ComputedStyle,
    /// The border box, in page coordinates.
    pub frame: Rect,
    /// For a box placed in a line: the distance from `frame.y` to its baseline.
    pub baseline: f32,
    /// Child boxes, in paint order. An anonymous box's children are the
    /// inline-level boxes its lines refer to.
    pub children: Vec<LayoutBox>,
    /// Lines of inline content, for an anonymous box.
    pub lines: Vec<Line>,
}

/// One line of inline content.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The line's box, after alignment.
    pub rect: Rect,
    /// Its pieces, left to right.
    pub fragments: Vec<Fragment>,
}

/// One piece of a line: a word, or an inline-level box.
#[derive(Debug, Clone, PartialEq)]
pub struct Fragment {
    /// Where the piece is, in page coordinates.
    pub rect: Rect,
    /// The distance from `rect.y` to the text baseline.
    pub baseline: f32,
    /// What the piece is.
    pub content: FragmentContent,
}

/// What a fragment holds.
#[derive(Debug, Clone, PartialEq)]
pub enum FragmentContent {
    /// A word of text.
    Text {
        /// The word.
        text: String,
        /// Its colour.
        color: Color,
        /// Its font size in CSS pixels.
        font_size: f32,
        /// Whether it is bold.
        bold: bool,
    },
    /// An inline-level box: an index into the line's box's `children`.
    Box(usize),
}

impl LayoutBox {
    /// Moves this box and everything in it by `(dx, dy)`.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.frame = self.frame.translated(dx, dy);
        for line in &mut self.lines {
            line.rect = line.rect.translated(dx, dy);
            for fragment in &mut line.fragments {
                fragment.rect = fragment.rect.translated(dx, dy);
            }
        }
        for child in &mut self.children {
            child.translate(dx, dy);
        }
    }

    /// The deepest box with an element under the point, if any.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&LayoutBox> {
        if !self.frame.contains(x, y) {
            return None;
        }
        self.children
            .iter()
            .rev()
            .find_map(|child| child.hit_test(x, y))
            .or(self.node.map(|_| self))
    }

    /// The distance from the top of this box to its first line's baseline, if it has any text.
    pub fn first_baseline(&self) -> Option<f32> {
        if let Some(fragment) = self.lines.first().and_then(|line| line.fragments.first()) {
            return Some(fragment.rect.y + fragment.baseline - self.frame.y);
        }
        self.children.iter().find_map(|child| {
            child
                .first_baseline()
                .map(|baseline| baseline + child.frame.y - self.frame.y)
        })
    }
}

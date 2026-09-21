//! Layout.
//!
//! Turns styled elements into positioned boxes. Block and flex layout come
//! from `taffy`; inline formatting (words, inline-blocks, line breaking) is
//! VPP's own.
//!
//! # Where it sits
//!
//! Between style and paint. It reads the DOM and computed styles, measures
//! text through [`TextMeasure`], and produces a [`LayoutBox`] tree in CSS
//! pixels.
//!
//! # Not in this crate
//!
//! Drawing, and fonts themselves: `vpp-paint` turns boxes and text runs into
//! pixels, and provides the real [`TextMeasure`].
//!
//! # Where to start reading
//!
//! [`layout_document`] in `document.rs`, then `layouter.rs`, which builds the
//! `taffy` tree (`taffy_style.rs`) and reads the result back as the tree in
//! `tree.rs`. `inline.rs` collects inline content and lays it out with
//! `line_break.rs`.
//!
//! # Differences from the C++ engine
//!
//! Block and flex layout follow the CSS specifications through `taffy`, where
//! the C++ engine approximated them. Visible differences: vertical margins
//! also collapse between a parent and its first or last child; flex items
//! shrink by the flexbox algorithm; and text directly inside a flex container
//! is laid out instead of dropped.

mod document;
mod geometry;
mod inline;
mod layouter;
mod line_break;
mod measure;
mod taffy_style;
mod tree;

pub use document::layout_document;
pub use geometry::Rect;
pub use measure::{FixedMeasure, TextMeasure};
pub use tree::{Fragment, FragmentContent, LayoutBox, Line};

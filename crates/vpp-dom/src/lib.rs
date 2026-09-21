//! The document tree.
//!
//! Holds a page as an arena of nodes linked by generational ids, parses HTML
//! into it with `html5ever`, and encodes it as `dom.bin`.
//!
//! # Where it sits
//!
//! Directly above `vpp-format`. Style, layout, script, and the template
//! compiler all work on this tree.
//!
//! # Not in this crate
//!
//! Styles, boxes, or pixels. Nodes know their tag, attributes, text, and
//! relatives; nothing else.
//!
//! # Where to start reading
//!
//! [`Document`] (`document.rs`) and [`Node`] (`node.rs`), then `tree.rs` for
//! every operation that changes the tree. `parse.rs` builds a document from
//! HTML, and `binary.rs` reads and writes `dom.bin` (`docs/formats/dom.md`).

mod binary;
mod document;
mod node;
mod parse;
mod query;
mod tree;

pub use binary::{DomBinError, MAX_DEPTH, decode_dom, encode_dom};
pub use document::Document;
pub use node::{Attribute, Element, Node, NodeData, NodeId};
pub use parse::parse_html;
pub use tree::Descendants;

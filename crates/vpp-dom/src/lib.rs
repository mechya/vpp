//! The document tree.
//!
//! Holds a page as an arena of nodes linked by generational ids, parses HTML into it with `html5ever`, and encodes it as `dom.bin`.
//!
//! # Where it sits
//!
//! Directly above `vpp-format`. Style, layout, script, and the template compiler all work on this tree.
//!
//! # Not in this crate
//!
//! Styles, boxes, or pixels. Nodes know their tag, attributes, text, and relatives; nothing else.
//!
//! # Status
//!
//! Not implemented yet. The removed C++ implementation was `runtime/src/dom.cpp`, `runtime/src/html.cpp`, and the `dom.bin` half of `runtime/src/binary.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

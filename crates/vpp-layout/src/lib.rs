//! Layout.
//!
//! Turns styled elements into positioned boxes. Block, flex, and grid come from `taffy`; inline formatting and line breaking are VPP's own.
//!
//! # Where it sits
//!
//! Between style and paint.
//!
//! # Not in this crate
//!
//! Drawing. Layout produces boxes and text runs with positions; `vpp-paint` turns them into pixels.
//!
//! # Status
//!
//! Not ported yet. The C++ reference implementation is `runtime/src/layout.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

//! JavaScript.
//!
//! Runs a page's compiled scripts through the `ScriptEngine` trait, with `rquickjs` as the engine, and exposes the DOM, `console`, and the `VPP.*` APIs to them.
//!
//! # Where it sits
//!
//! Next to the DOM: scripts read and change the tree, and the viewer re-runs style and layout when they do. The compiler also uses the engine to produce `code.bin`.
//!
//! # Not in this crate
//!
//! Deciding what a page may do beyond its API level and permissions, which are set by the viewer.
//!
//! # Status
//!
//! Not ported yet. The C++ reference implementation is `runtime/src/script.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

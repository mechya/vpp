//! CSS.
//!
//! Parses stylesheets (tokenising with `cssparser`), encodes them as `style.bin`, matches selectors, runs the cascade, and produces a computed style for every element.
//!
//! # Where it sits
//!
//! Between the DOM and layout. The compiler uses it to produce `style.bin`; the viewer uses it before every layout.
//!
//! # Not in this crate
//!
//! Box sizes and positions. Those are `vpp-layout`'s job.
//!
//! # Status
//!
//! Not implemented yet. The removed C++ implementation was `runtime/src/css.cpp`, `runtime/src/style.cpp`, and the `style.bin` half of `runtime/src/binary.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

//! Build-time templates.
//!
//! Expands layouts, includes, slots, and components (`docs/template-syntax.md`) into one plain HTML page, with errors that point at `file:line:col`.
//!
//! # Where it sits
//!
//! Used only by the compiler. The viewer never sees a template.
//!
//! # Not in this crate
//!
//! Runtime templating (`{{ }}`, bindings). That needs its own design document first.
//!
//! # Status
//!
//! Not ported yet. The C++ reference implementation is `runtime/src/template.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

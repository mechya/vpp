//! JavaScript.
//!
//! Checks a page's scripts at build time, and runs them in the viewer with
//! QuickJS through `rquickjs`, exposing the DOM, `console`, and the `VPP.*`
//! APIs to them.
//!
//! # Where it sits
//!
//! Next to the DOM: scripts read and change the tree, and the viewer re-runs
//! style and layout when they do. The compiler uses it to check every script
//! before packaging it.
//!
//! # Not in this crate
//!
//! The `code.bin` container, which is plain data (`vpp-format`). Carrying out
//! what scripts ask for (a popup, closing the window): scripts only queue
//! [`HostRequest`]s, and the viewer acts on them.
//!
//! # Where to start reading
//!
//! [`ScriptEngine`] in `engine.rs`, then [`QuickJsEngine`] in `quickjs.rs`.
//! The page's API is built in JavaScript in `prelude.js`, on the small native
//! interface in `bindings.rs`. `syntax.rs` is the build-time syntax check.
//!
//! # Unsafe code
//!
//! This is one of the few crates allowed `unsafe`, at the engine boundary
//! only: its `Cargo.toml` denies it everywhere except functions marked
//! `#[allow(unsafe_code)]`, each use with a `// SAFETY:` comment. Today that is
//! only the compile-only call in `syntax.rs`.

mod bindings;
mod engine;
mod quickjs;
mod syntax;

pub use engine::{HostRequest, ScriptEngine, ScriptError};
pub use quickjs::{MEMORY_LIMIT, QuickJsEngine, TIME_LIMIT};
pub use syntax::{SyntaxError, check_syntax};

//! `vppc`, the VPP compiler.
//!
//! Compiles a page and its layouts, components, CSS, and JavaScript into `dom.bin`, `style.bin`, and `code.bin`.
//!
//! # Where it sits
//!
//! The first development step. Its output goes to the packager.
//!
//! # Not in this crate
//!
//! Signing and publishing (`vpp-packager`).
//!
//! # Status
//!
//! Not ported yet. The C++ reference implementation is `compiler/src/main.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("vppc: not ported to Rust yet; use the C++ build (build.cmd) for now");
    ExitCode::FAILURE
}

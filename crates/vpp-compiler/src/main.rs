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
//! Not implemented yet. The removed C++ implementation was `compiler/src/main.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("vppc: not implemented yet; the Rust port is in progress (docs/rust-port.md)");
    ExitCode::FAILURE
}

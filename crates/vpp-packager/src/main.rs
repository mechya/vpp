//! `vpppack`, the VPP packager.
//!
//! Creates publisher keys, and hashes, signs, and publishes compiled pages as `.vpp` packages, `.vppm` manifests, and a shared `res/` folder.
//!
//! # Where it sits
//!
//! The second development step. Its output is what a static server hosts.
//!
//! # Not in this crate
//!
//! Compiling pages (`vpp-compiler`).
//!
//! # Status
//!
//! Not implemented yet. The removed C++ implementation was `packager/src/main.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("vpppack: not implemented yet; the Rust port is in progress (docs/rust-port.md)");
    ExitCode::FAILURE
}

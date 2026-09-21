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
//! Not ported yet. The C++ reference implementation is `packager/src/main.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("vpppack: not ported to Rust yet; use the C++ build (build.cmd) for now");
    ExitCode::FAILURE
}

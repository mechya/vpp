//! `vpp-viewer`, the VPP viewer.
//!
//! Opens a window, draws the shell (back, forward, reload, address, window buttons), loads and verifies pages, runs them, and navigates between them.
//!
//! # Where it sits
//!
//! The top of the stack. It depends on every library crate and nothing depends on it.
//!
//! # Not in this crate
//!
//! Trust decisions (`vpp-updater`) and rendering stages (`vpp-style`, `vpp-layout`, `vpp-paint`). The viewer wires them together.
//!
//! # Status
//!
//! Not ported yet. The C++ reference implementation is `viewer/src/main.cpp` and `viewer/src/shell.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("vpp-viewer: not ported to Rust yet; use the C++ build (build.cmd) for now");
    ExitCode::FAILURE
}

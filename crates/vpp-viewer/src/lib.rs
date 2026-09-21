//! The VPP viewer.
//!
//! The whole viewer as a library: the shell (back, forward, reload, address, window buttons), loading and verifying pages, running them, navigating between them, the rendering loop, and input.
//!
//! # Where it sits
//!
//! Above every other library crate. The entry-point crates `vpp-desktop`, `vpp-android`, and `vpp-ios` start it; it contains no operating-system code of its own.
//!
//! # Not in this crate
//!
//! Trust decisions (`vpp-updater`), rendering stages (`vpp-style`, `vpp-layout`, `vpp-paint`), and operating-system services (`vpp-platform`). The viewer wires them together.
//!
//! # Status
//!
//! Not implemented yet. The removed C++ implementation was `viewer/src/main.cpp` and `viewer/src/shell.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

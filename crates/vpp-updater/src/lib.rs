//! Page store and updates.
//!
//! Keeps visited pages on disk, pins each site's publisher key on first use, checks manifests, and downloads only the resources whose hashes changed. It refuses older versions.
//!
//! # Where it sits
//!
//! Between the network and the viewer. Every page the viewer opens from a URL comes through here.
//!
//! # Not in this crate
//!
//! The byte formats themselves (`vpp-format`).
//!
//! # Status
//!
//! Not implemented yet. The removed C++ implementation was `updater/src/http.cpp`, `updater/src/updater.cpp`, and the publisher-trust section of `viewer/src/main.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

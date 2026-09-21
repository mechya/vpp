//! Binary formats of VPP.
//!
//! Reads and writes the VPP containers: the `.vpp` page package, the `.vppm` update manifest, the `code.bin` container, and `vpp.json` site configuration. It also owns the byte reader and writer, every format version number, publisher keys, Ed25519 signatures, and SHA-256 resource hashes.
//!
//! # Where it sits
//!
//! The bottom of the stack. Every other crate that touches a file depends on it, and it depends on no VPP crate. Tools such as a package inspector can use it on its own.
//!
//! # Not in this crate
//!
//! The `dom.bin` and `style.bin` codecs, which live next to the types they encode (`vpp-dom`, `vpp-style`) and use this crate's byte reader and version numbers. Networking and the page store (`vpp-updater`). This crate encodes and decodes; it does not decide what to trust.
//!
//! # Status
//!
//! Not implemented yet. The removed C++ implementation was `runtime/src/package.cpp`, `crypto.cpp`, `sha256.cpp`, and `runtime/include/vpp/bytes.h`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

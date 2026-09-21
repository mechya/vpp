//! Binary formats of VPP.
//!
//! Reads and writes the VPP containers: the `.vpp` page package, the `.vppm`
//! update manifest, the `code.bin` script container, and `vpp.json` site
//! configuration. It also owns the byte
//! reader and writer, every format version number, publisher keys, Ed25519
//! signatures, and SHA-256 resource hashes.
//!
//! # Where it sits
//!
//! The bottom of the stack. Every other crate that touches a file depends on
//! it, and it depends on no VPP crate. Tools such as a package inspector can use
//! it on its own.
//!
//! # Not in this crate
//!
//! The `dom.bin` and `style.bin` codecs, which live next to the types they
//! encode (`vpp-dom`, `vpp-style`) and use this crate's byte reader and version
//! numbers. Compiling or running JavaScript (`vpp-script`). Networking,
//! the page store, and trust decisions (`vpp-updater`): this crate says whether
//! a signature is valid, not whether its publisher should be trusted.
//!
//! # Where to start reading
//!
//! [`Package`] in `package.rs`, with `docs/formats/package.md` beside it. It is
//! built from [`ByteWriter`] and [`ByteReader`] (`bytes.rs`), [`ResourceHash`]
//! (`hash.rs`), and [`SigningKey`] and [`PublicKey`] (`keys.rs`, `signature.rs`).
//! Format versions are in `version.rs`.

mod bytes;
mod code_bin;
mod hash;
mod hex;
mod keys;
mod manifest;
mod package;
mod signature;
mod site_config;
mod version;

pub use bytes::{ByteReader, ByteWriter, DecodeError, MAX_COUNT, MAX_STRING};
pub use code_bin::{CodeBin, CodeBinError, ScriptKind};
pub use hash::ResourceHash;
pub use keys::{KeyError, PublicKey, SigningKey};
pub use manifest::Manifest;
pub use package::{Package, PackageEntry, PackageError, SignatureStatus};
pub use signature::Signature;
pub use site_config::{SiteConfig, SiteConfigError};
pub use version::{
    CODE_MAGIC, CODE_VERSION, DOM_MAGIC, DOM_VERSION, PACKAGE_MAGIC, PACKAGE_VERSION, STYLE_MAGIC,
    STYLE_VERSION,
};

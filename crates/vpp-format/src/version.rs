//! Magic bytes and version numbers of every VPP binary format, in one place.
//!
//! The rules for changing a format are in `docs/versioning.md`, section 3: any
//! change to a byte layout bumps its version here, adds a line to its history,
//! adds a fixture in `tests/fixtures/`, and updates `docs/formats/`.

/// Magic bytes at the start of a `.vpp` package and a `.vppm` manifest.
pub const PACKAGE_MAGIC: [u8; 4] = *b"VPPK";

/// Package format version: the one this crate writes, and the only one it reads.
///
/// History: 1, first format. 2, adds the page name. 3, adds the `window`
/// preferences from `vpp.json`.
pub const PACKAGE_VERSION: u16 = 3;

/// Magic bytes at the start of `dom.bin`, encoded by `vpp-dom`.
pub const DOM_MAGIC: [u8; 4] = *b"VPPD";

/// `dom.bin` format version.
///
/// History: 1, first format.
pub const DOM_VERSION: u16 = 1;

/// Magic bytes at the start of a `style.bin` resource, encoded by `vpp-style`.
pub const STYLE_MAGIC: [u8; 4] = *b"VPPS";

/// `style.bin` format version.
///
/// History: 1, first format.
pub const STYLE_VERSION: u16 = 1;

/// Magic bytes at the start of a `code.bin` script resource.
pub const CODE_MAGIC: [u8; 4] = *b"VPPC";

/// `code.bin` format version.
///
/// History: 1, first format: JavaScript source, not bytecode
/// (`docs/design/0001-code-bin.md`). The C++ tools wrote raw bytecode with no header.
pub const CODE_VERSION: u16 = 1;

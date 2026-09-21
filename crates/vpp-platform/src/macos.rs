//! macOS services for `vpp-platform`.
//!
//! Later: starts when macOS work begins (`docs/rust-port.md` §8). Not compiled on other systems.

use std::path::PathBuf;

use crate::FontFiles;

/// Not yet: filled in when macOS work begins.
pub(crate) fn data_dir() -> Option<PathBuf> {
    None
}

/// Not yet: filled in when macOS work begins.
pub(crate) fn clipboard_text() -> Option<String> {
    None
}

/// Not yet: filled in when macOS work begins.
pub(crate) fn font_candidates() -> Vec<FontFiles> {
    Vec::new()
}

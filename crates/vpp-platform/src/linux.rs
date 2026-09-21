//! Linux (X11 and Wayland) services for `vpp-platform`.
//!
//! Later: starts when Linux work begins (`docs/rust-port.md` §8). Not compiled on other systems.

use std::path::PathBuf;

use crate::FontFiles;

/// Not yet: filled in when Linux work begins.
pub(crate) fn data_dir() -> Option<PathBuf> {
    None
}

/// Not yet: filled in when Linux work begins.
pub(crate) fn clipboard_text() -> Option<String> {
    None
}

/// Not yet: filled in when Linux work begins.
pub(crate) fn font_candidates() -> Vec<FontFiles> {
    Vec::new()
}

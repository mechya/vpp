//! iOS services for `vpp-platform`.
//!
//! Later, and only if the App Store check in `docs/rust-port.md` §4.1 allows it. Reaches Apple APIs through `objc2`. Not compiled on other systems.

use std::path::PathBuf;

use crate::FontFiles;

/// Not yet: filled in with the iOS app.
pub(crate) fn data_dir() -> Option<PathBuf> {
    None
}

/// Not yet: filled in with the iOS app.
pub(crate) fn clipboard_text() -> Option<String> {
    None
}

/// Not yet: filled in with the iOS app.
pub(crate) fn font_candidates() -> Vec<FontFiles> {
    Vec::new()
}

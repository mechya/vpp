//! Android services for `vpp-platform`.
//!
//! Later: starts with the Android app (`docs/rust-port.md` §8). Reaches Android APIs through `jni`. Not compiled on other systems.

use std::path::PathBuf;

use crate::FontFiles;

/// Not yet: filled in with the Android app.
pub(crate) fn data_dir() -> Option<PathBuf> {
    None
}

/// Not yet: filled in with the Android app.
pub(crate) fn clipboard_text() -> Option<String> {
    None
}

/// Not yet: filled in with the Android app.
pub(crate) fn font_candidates() -> Vec<FontFiles> {
    Vec::new()
}

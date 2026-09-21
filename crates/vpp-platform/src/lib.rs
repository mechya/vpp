//! Operating-system services.
//!
//! Gives the viewer one API for what differs per operating system: data, cache, and config folders, system fonts, the clipboard, opening a URL in the system browser, the form factor (desktop or mobile), and registering `.vpp` files and links.
//!
//! # Where it sits
//!
//! Used only by `vpp-viewer` and the entry-point crates. Library crates are given folders and services as arguments instead, so they never depend on this crate.
//!
//! # Not in this crate
//!
//! Windows, input, and the event loop, which `winit` provides. Anything that is not Rust, such as installers and app projects, lives in `platforms/<os>/`.
//!
//! # Status
//!
//! Windows works: the data folder, the system fonts, and reading the clipboard (`windows.rs`). `macos.rs`, `linux.rs`, `android.rs`, and `ios.rs` are placeholders that find nothing yet. The removed C++ viewer kept its folders in `prefDir` (`git show 1d1cd11:viewer/src/main.cpp`).

use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as os;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
use android as os;

#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "ios")]
use ios as os;

/// A font family's files: the regular face, and the bold face if there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFiles {
    /// The regular face.
    pub regular: PathBuf,
    /// The bold face.
    pub bold: Option<PathBuf>,
}

/// The viewer's own data folder, for pinned publisher keys and installed
/// sites. It may not exist yet. `None` if the system does not say where.
pub fn data_dir() -> Option<PathBuf> {
    os::data_dir()
}

/// The text on the clipboard, if there is any.
pub fn clipboard_text() -> Option<String> {
    os::clipboard_text()
}

/// The system's fonts for page text, best first. Some may be missing; the
/// viewer uses the first that loads.
pub fn font_candidates() -> Vec<FontFiles> {
    os::font_candidates()
}

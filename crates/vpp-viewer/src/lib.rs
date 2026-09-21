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
//! Trust decisions (`vpp-updater`), rendering stages (`vpp-style`, `vpp-layout`, `vpp-paint`), and operating-system services (`vpp-platform`). The viewer wires them together. The window itself belongs to the entry point, which passes input to a [`Viewer`] and shows the pixels it draws.
//!
//! # Where to start reading
//!
//! [`Viewer`] in `app.rs`. A page is loaded by `page_loader.rs` (from a [`Location`](location) in `location.rs`), started and clicked in `page.rs`, and drawn by `render.rs` below the shell bar (`shell.rs`, shaped by `window_prefs.rs`), with `popup.rs` on top. `input.rs` takes the keyboard and says where the frameless window drags and resizes. `navigation.rs` keeps the history and resolves links.
//!
//! # Status
//!
//! Works on Windows through `vpp-desktop` (`docs/rust-port.md` §8, step 7), with the shell bar, the address field, and the `vpp.json` window options. The removed C++ implementation was `viewer/src/main.cpp` and `viewer/src/shell.cpp`; read it with `git show 7cf865e:<path>`, and its behaviour in `docs/reference/viewer.md`.

mod app;
mod input;
mod location;
mod navigation;
mod page;
mod page_loader;
mod popup;
mod render;
mod shell;
#[cfg(test)]
mod tests;
mod window_prefs;

pub use app::{Viewer, ViewerOptions, WindowCommand};
pub use input::{Key, Modifiers, ResizeEdge, WindowRegion};
pub use vpp_paint::{Font, FontSet, Pixmap};
pub use vpp_updater::{Fetch, FetchError, HttpFetcher};

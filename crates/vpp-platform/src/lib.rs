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
//! Windows comes first, with the desktop viewer (`docs/rust-port.md` §8, step 7); `windows.rs` is its home. `macos.rs`, `linux.rs`, `android.rs`, and `ios.rs` are placeholders for later. The removed C++ viewer kept its folders in `prefDir` (`git show 1d1cd11:viewer/src/main.cpp`).

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;

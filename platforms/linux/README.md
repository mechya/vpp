# VPP on Linux

**Later.** Starts after the Windows desktop viewer works (`docs/rust-port.md` §8).

Everything Linux-specific that is **not Rust** lives in this folder. The Rust side is `crates/vpp-desktop` (shared with Windows and macOS) and `crates/vpp-platform/src/linux.rs`. No other crate contains Linux-specific code (`ARCHITECTURE.md`, "Platforms").

## What goes here

* the `.desktop` file and icons, generated from `platforms/assets/`
* the MIME type for `.vpp` files and the `vpp://` link handler
* packaging: AppImage or Flatpak (to be decided)

## Tools needed

Rust, plus the X11 and Wayland development packages `winit` needs (listed here when Linux work starts).

## Build

```sh
cargo build --release -p vpp-desktop
```

## Targets

x86_64 first; aarch64 later. Both X11 and Wayland.

# VPP on macOS

**Later.** Starts after the Windows desktop viewer works (`docs/rust-port.md` §8).

Everything macOS-specific that is **not Rust** lives in this folder. The Rust side is `crates/vpp-desktop` (shared with Windows and Linux) and `crates/vpp-platform/src/macos.rs`. No other crate contains macOS-specific code (`ARCHITECTURE.md`, "Platforms").

## What goes here

* `Info.plist` and the `.app` bundle layout
* the application icon (`.icns`), generated from `platforms/assets/`
* entitlements, code signing, and notarisation
* the disk image (`.dmg`) for distribution
* registration of `.vpp` files and `vpp://` links

## Tools needed

Rust and Xcode's command-line tools, on a Mac.

## Build

```sh
cargo build --release -p vpp-desktop
```

Bundling into a `.app` is added here when macOS work starts.

## Targets

aarch64 (Apple silicon) and x86_64, possibly as one universal binary.

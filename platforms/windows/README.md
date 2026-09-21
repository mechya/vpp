# VPP on Windows

**Current focus.** The first platform VPP ships on.

Everything Windows-specific that is **not Rust** lives in this folder. The Rust side is `crates/vpp-desktop` (the `vpp-viewer` executable) and `crates/vpp-platform/src/windows.rs`. No other crate contains Windows-specific code (`ARCHITECTURE.md`, "Platforms").

## What goes here

* the installer (format to be decided: MSIX or MSI)
* the application icon (`.ico`), generated from `platforms/assets/`
* the application manifest: DPI awareness, supported Windows versions
* registration of `.vpp` files and `vpp://` links
* code signing of the executable and installer

## Tools needed

Rust (`rust-toolchain.toml` installs the right version) and the Visual Studio C++ build tools, which Rust uses for linking.

## Build

```sh
cargo build --release -p vpp-desktop
```

The executable is `target\release\vpp-viewer.exe`.

## Targets

x86_64 first; aarch64 (Windows on Arm) later.

# VPP on iOS

**Later, and only if allowed.** Apple's App Store rules on apps that download and run code, and on web-browsing apps, may rule VPP out. Read the check in `docs/rust-port.md` §4.1 before starting.

Everything iOS-specific that is **not Rust** lives in this folder. The Rust side is `crates/vpp-ios` (builds `libvpp_ios.a`) and `crates/vpp-platform/src/ios.rs`. No other crate contains iOS-specific code (`ARCHITECTURE.md`, "Platforms").

## What goes here

* the Xcode project and the Swift entry point that calls into `libvpp_ios.a`
* `Info.plist`, including the document types for `.vpp` files and the `vpp://` URL scheme
* app icons, generated from `platforms/assets/`
* signing and provisioning settings (never the certificates themselves)

## Tools needed

Rust with the iOS targets, and Xcode, on a Mac.

## Build

Added when iOS work starts. It will build the static library for the device and simulator targets and link it from Xcode.

## Targets

aarch64 devices; aarch64 simulator.

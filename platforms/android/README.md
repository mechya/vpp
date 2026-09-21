# VPP on Android

**Later.** Starts after the Windows desktop viewer works (`docs/rust-port.md` §8).

Everything Android-specific that is **not Rust** lives in this folder. The Rust side is `crates/vpp-android` (builds `libvpp_android.so`) and `crates/vpp-platform/src/android.rs`. No other crate contains Android-specific code (`ARCHITECTURE.md`, "Platforms").

## What goes here

* the Gradle project: `settings.gradle`, `app/build.gradle`
* `AndroidManifest.xml`, including the intent filters for `.vpp` files and `vpp://` links
* the activity that loads `libvpp_android.so`
* launcher icons, generated from `platforms/assets/`
* release signing configuration (never the keys themselves)

## Tools needed

Rust with the Android targets, `cargo-ndk`, Android Studio, and the Android NDK.

## Build

Added when Android work starts. It will build the native library with `cargo ndk` and then the app with Gradle.

## Targets

arm64-v8a for phones; x86_64 for the emulator.

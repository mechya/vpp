# vpp-android

Android entry point.

Builds `libvpp_android.so`, which the Android app in `platforms/android/` loads. It starts the viewer from `android_main`.

**Where it sits:** An entry point: nothing depends on it.

**Not in this crate:** The Gradle project, manifest, and packaging, which live in `platforms/android/`.

**Depends on:** `vpp-viewer`, `vpp-platform`.

## Status

Later: Android starts after the Windows desktop viewer (`docs/rust-port.md` §8). Until then this crate is an empty placeholder.

## Test it on its own

See `platforms/android/README.md`. Until Android work starts there is nothing to run.

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

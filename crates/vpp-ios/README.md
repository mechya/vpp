# vpp-ios

iOS entry point.

Builds `libvpp_ios.a`, which the iOS app in `platforms/ios/` links. It starts the viewer from a C entry function called by the Swift app.

**Where it sits:** An entry point: nothing depends on it.

**Not in this crate:** The Xcode project, Info.plist, and signing, which live in `platforms/ios/`.

**Depends on:** `vpp-viewer`, `vpp-platform`.

## Status

Later, and only if the App Store check in `docs/rust-port.md` §4.1 allows it. Until then this crate is an empty placeholder.

## Test it on its own

See `platforms/ios/README.md`. Until iOS work starts there is nothing to run.

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

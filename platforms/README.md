# Platforms

Everything per operating system that is not Rust: app projects, manifests, icons, installers, and signing. Rust code that differs per OS lives only in `crates/vpp-platform` (one file per OS) and the entry-point crates (`vpp-desktop`, `vpp-android`, `vpp-ios`). See `ARCHITECTURE.md`, "Platforms", and `docs/rust-port.md` §4.1.

| Folder | Status | Entry-point crate |
|---|---|---|
| [windows/](windows/) | **Current focus** | `vpp-desktop` |
| [macos/](macos/) | Later | `vpp-desktop` |
| [linux/](linux/) | Later | `vpp-desktop` |
| [android/](android/) | Later | `vpp-android` |
| [ios/](ios/) | Later, if App Store rules allow | `vpp-ios` |
| [assets/](assets/) | Shared source artwork | — |

Secrets never go in these folders: signing keys, certificates, and provisioning profiles stay with the lead maintainer.

# vpp-platform

Operating-system services.

Gives the viewer one API for what differs per operating system: data, cache, and config folders, system fonts, the clipboard, opening a URL in the system browser, the form factor (desktop or mobile), and registering `.vpp` files and links.

**Where it sits:** Used only by `vpp-viewer` and the entry-point crates. Library crates are given folders and services as arguments instead, so they never depend on this crate.

**Not in this crate:** Windows, input, and the event loop, which `winit` provides. Anything that is not Rust, such as installers and app projects, lives in `platforms/<os>/`.

**Depends on:** no other VPP crate.

## Status

Windows works: `data_dir()` is `%LOCALAPPDATA%\VPP\Viewer`, `font_candidates()` lists Segoe UI, then Arial, and `clipboard_text()` reads the clipboard through `arboard` (`src/windows.rs`). `macos.rs`, `linux.rs`, `android.rs`, and `ios.rs` are placeholders that find nothing yet. The removed C++ viewer kept its folders in `prefDir` (`git show 7cf865e:viewer/src/main.cpp`).

## Test it on its own

```sh
cargo test -p vpp-platform
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

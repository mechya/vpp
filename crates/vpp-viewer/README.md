# vpp-viewer

The VPP viewer.

The whole viewer as a library: the shell (back, forward, reload, address, window buttons), loading and verifying pages, running them, navigating between them, the rendering loop, and input.

**Where it sits:** Above every other library crate. The entry-point crates `vpp-desktop`, `vpp-android`, and `vpp-ios` start it; it contains no operating-system code of its own.

**Not in this crate:** Trust decisions (`vpp-updater`), rendering stages (`vpp-style`, `vpp-layout`, `vpp-paint`), and operating-system services (`vpp-platform`). The viewer wires them together. The window itself belongs to the entry point, which passes input to a `Viewer` and shows the pixels it draws.

**Depends on:** `vpp-format`, `vpp-template` (development mode), `serde_json` (window options), `vpp-dom`, `vpp-style`, `vpp-layout`, `vpp-paint`, `vpp-script`, `vpp-updater`, `vpp-platform`.

## Status

Works on Windows through `vpp-desktop` (`docs/rust-port.md` §8, step 7). It opens a page's HTML source, a `dist` folder, a signed `.vpp` package, or a URL; checks packages before anything runs; runs the page's scripts; handles clicks, links, history, and `VPP.window.popup`; and draws each frame into a pixel buffer. It draws its own shell bar (`src/shell.rs`) shaped by the site's `vpp.json` window options (`src/window_prefs.rs`), takes the keyboard and the address field, and tells the entry point where the frameless window drags and resizes (`src/input.rs`). Start reading at `Viewer` in `src/app.rs`. The removed C++ implementation was `viewer/src/main.cpp` and `viewer/src/shell.cpp`; read it with `git show 7cf865e:<path>`, and its behaviour in `docs/reference/viewer.md`.

Clicks find the deepest element with a box. A link inside running text (an inline `<a>` in a paragraph) has no box of its own, so clicking it does not follow it yet, as in the C++ viewer; links laid out as blocks or flex items, like the examples' menus, work.

## Test it on its own

```sh
cargo test -p vpp-viewer
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

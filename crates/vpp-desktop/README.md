# vpp-desktop

`vpp-viewer`, the desktop viewer.

Starts the viewer on the desktop: reads the command line, creates the window's event loop, and hands it to `vpp-viewer`.

**Where it sits:** An entry point: nothing depends on it. Windows first; macOS and Linux use this same crate later.

**Not in this crate:** Anything the viewer does once it runs (`vpp-viewer`), and operating-system services (`vpp-platform`).

**Depends on:** `vpp-viewer`, `vpp-platform`, `winit` (the window and input), `softbuffer` (showing pixels).

## Status

Works on Windows (`docs/rust-port.md` §8, step 7b): `vpp-viewer [ADDRESS] [--allow-unsigned]` opens a frameless window with the viewer's own shell bar, and passes it the mouse and keyboard. `src/cli.rs` reads the command line; `src/window.rs` passes window events to the viewer, starts system drags and resizes where the viewer says, and carries out its commands; `src/keys.rs` translates keys; `src/present.rs` copies each frame into the window. Rounded corners use Windows 11's own (`set_rounded_corners`, the one Windows-only function here). A release build is a windowed program with no console; a debug build (`cargo run`) keeps the console for the log. The log (what was loaded and verified, and `console.log`) is printed to the console. The removed C++ implementation was `viewer/src/main.cpp`; read it with `git show 1d1cd11:<path>`.

## Run it

```sh
cargo run -p vpp-desktop
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

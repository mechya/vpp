# vpp-desktop

`vpp-viewer`, the desktop viewer.

Starts the viewer on the desktop: reads the command line, creates the window's event loop, and hands it to `vpp-viewer`.

**Where it sits:** An entry point: nothing depends on it. Windows first; macOS and Linux use this same crate later.

**Not in this crate:** Anything the viewer does once it runs (`vpp-viewer`), and operating-system services (`vpp-platform`).

**Depends on:** `vpp-viewer`, `vpp-platform`.

## Status

Not implemented yet. The removed C++ implementation was the `main` function and argument handling in `viewer/src/main.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

## Run it

```sh
cargo run -p vpp-desktop
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

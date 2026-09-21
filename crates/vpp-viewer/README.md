# vpp-viewer

`vpp-viewer`, the VPP viewer.

Opens a window, draws the shell (back, forward, reload, address, window buttons), loads and verifies pages, runs them, and navigates between them.

**Where it sits:** The top of the stack. It depends on every library crate and nothing depends on it.

**Not in this crate:** Trust decisions (`vpp-updater`) and rendering stages (`vpp-style`, `vpp-layout`, `vpp-paint`). The viewer wires them together.

**Depends on:** `vpp-format`, `vpp-dom`, `vpp-style`, `vpp-layout`, `vpp-paint`, `vpp-script`, `vpp-updater`.

## Status

Not ported yet. The C++ reference implementation is `viewer/src/main.cpp` and `viewer/src/shell.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

## Run it

```sh
cargo run -p vpp-viewer
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

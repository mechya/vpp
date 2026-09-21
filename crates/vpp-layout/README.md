# vpp-layout

Layout.

Turns styled elements into positioned boxes. Block, flex, and grid come from `taffy`; inline formatting and line breaking are VPP's own.

**Where it sits:** Between style and paint.

**Not in this crate:** Drawing. Layout produces boxes and text runs with positions; `vpp-paint` turns them into pixels.

**Depends on:** `vpp-dom`, `vpp-style`.

## Status

Not implemented yet. The removed C++ implementation was `runtime/src/layout.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

## Test it on its own

```sh
cargo test -p vpp-layout
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

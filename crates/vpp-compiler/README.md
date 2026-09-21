# vpp-compiler

`vppc`, the VPP compiler.

Compiles a page and its layouts, components, CSS, and JavaScript into `dom.bin`, `style.bin`, and `code.bin`.

**Where it sits:** The first development step. Its output goes to the packager.

**Not in this crate:** Signing and publishing (`vpp-packager`).

**Depends on:** `vpp-format`, `vpp-template`, `vpp-dom`, `vpp-style`, `vpp-script`.

## Status

Not ported yet. The C++ reference implementation is `compiler/src/main.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

## Run it

```sh
cargo run -p vpp-compiler
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

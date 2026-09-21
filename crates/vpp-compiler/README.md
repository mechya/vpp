# vpp-compiler

`vppc`, the VPP compiler.

Compiles a page and its layouts, components, CSS, and JavaScript into `dom.bin`, `style.bin`, and `code.bin`.

**Where it sits:** The first development step. Its output goes to the packager.

**Not in this crate:** Signing and publishing (`vpp-packager`).

**Depends on:** `vpp-format`, `vpp-template`, `vpp-dom`, `vpp-style`, `vpp-script`.

## Status

Implemented (`docs/rust-port.md` §8, step 3). It replaces the C++ `compiler/src/main.cpp`, with the same command line. Pages and stylesheets compile byte for byte as the C++ tool did. Scripts are checked for syntax with QuickJS and shipped as source in `code.bin` (`docs/design/0001-code-bin.md`); `-g` is accepted and ignored.

`cli.rs` holds the command line and `compile.rs` the compiling.

## Run it

```sh
cargo run -p vpp-compiler
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

# vpp-script

JavaScript.

Checks and runs a page's JavaScript with QuickJS, through `rquickjs`, and exposes the DOM, `console`, and the `VPP.*` APIs to it.

**Where it sits:** Next to the DOM: scripts read and change the tree, and the viewer re-runs style and layout when they do. The compiler uses it to check every script before packaging it.

**Not in this crate:** Deciding what a page may do beyond its API level and permissions, which are set by the viewer.

**Depends on:** `vpp-dom`, `vpp-format` (`code.bin`).

## Status

Implemented. Start with the `ScriptEngine` trait (`src/engine.rs`) and its QuickJS backend (`src/quickjs.rs`). The page's API is written in JavaScript in `src/prelude.js`, on the checked natives in `src/bindings.rs`. `src/syntax.rs` is the syntax check `vppc` uses, with the crate's only `unsafe` code, a compile-only call into QuickJS. Each page gets its own runtime with a memory limit (`MEMORY_LIMIT`) and a time limit per script run and per event (`TIME_LIMIT`). Scripts never act on the window directly: they queue `HostRequest`s for the viewer. The C++ reference is `runtime/src/script.cpp` (`git show 1d1cd11:<path>`).

## Test it on its own

```sh
cargo test -p vpp-script
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

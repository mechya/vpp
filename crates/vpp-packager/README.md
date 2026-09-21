# vpp-packager

`vpppack`, the VPP packager.

Creates publisher keys, and hashes, signs, and publishes compiled pages as `.vpp` packages, `.vppm` manifests, and a shared `res/` folder.

**Where it sits:** The second development step. Its output is what a static server hosts.

**Not in this crate:** Compiling pages (`vpp-compiler`).

**Depends on:** `vpp-format`.

## Status

Not implemented yet. The removed C++ implementation was `packager/src/main.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

## Run it

```sh
cargo run -p vpp-packager
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

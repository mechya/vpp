# vpp-packager

`vpppack`, the VPP packager.

Creates publisher keys, and hashes, signs, and publishes compiled pages as `.vpp` packages, `.vppm` manifests, and a shared `res/` folder.

**Where it sits:** The second development step. Its output is what a static server hosts.

**Not in this crate:** Compiling pages (`vpp-compiler`).

**Depends on:** `vpp-format`.

## Status

Implemented (`docs/rust-port.md` §8, step 3). It replaces the C++ `packager/src/main.cpp`, with the same command line and output.

| File | Holds |
|---|---|
| `cli.rs` | The command line |
| `publish.rs` | Packaging a site, and `--publish`. Start here. |
| `keygen.rs` | `vpppack keygen` |
| `inspect.rs` | `vpppack --inspect` |

## Run it

```sh
cargo run -p vpp-packager
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

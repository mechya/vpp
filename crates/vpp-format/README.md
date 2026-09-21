# vpp-format

Binary formats of VPP.

Reads and writes the VPP containers: the `.vpp` page package, the `.vppm` update manifest, the `code.bin` script container, and `vpp.json` site configuration. It also owns the byte reader and writer, every format version number, publisher keys, Ed25519 signatures, and SHA-256 resource hashes.

**Where it sits:** The bottom of the stack. Every other crate that touches a file depends on it, and it depends on no VPP crate. Tools such as a package inspector can use it on its own.

**Not in this crate:** The `dom.bin` and `style.bin` codecs, which live next to the types they encode (`vpp-dom`, `vpp-style`) and use this crate's byte reader and version numbers. Compiling or running JavaScript (`vpp-script`). Networking and the page store (`vpp-updater`). This crate encodes and decodes; it does not decide what to trust.

**Depends on:** no other VPP crate.

## Status

Implemented (`docs/rust-port.md` §8, step 2). It replaces the C++ `runtime/src/package.cpp`, `crypto.cpp`, `sha256.cpp`, and `runtime/include/vpp/bytes.h` (`git show 1d1cd11:<path>`).

| File | Holds |
|---|---|
| `package.rs` | `Package`: build, open, verify, read, and assemble `.vpp` and `.vppm` files. Start here, with `docs/formats/package.md`. |
| `manifest.rs` | `Manifest`: the signed page description |
| `site_config.rs` | `SiteConfig`: `vpp.json` |
| `keys.rs`, `signature.rs` | Publisher keys, their text files, signing and verifying |
| `hash.rs` | `ResourceHash`: SHA-256 |
| `bytes.rs` | `ByteReader` and `ByteWriter`, with the decoder limits |
| `version.rs` | Magic bytes and version numbers of every format |
| `code_bin.rs` | `CodeBin`: a script's source in its container (`docs/formats/code.md`) |
| `hex.rs` | Hex encoding for key files and resource names |

The frozen version 3 fixtures in `tests/fixtures/v3/` are checked by `tests/fixtures_v3.rs`, and were written once by `examples/write_fixtures.rs`.

## Test it on its own

```sh
cargo test -p vpp-format
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

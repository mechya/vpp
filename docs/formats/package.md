# `.vpp` page package and `.vppm` manifest

**Current version:** 3
**Implementation:** `crates/vpp-format/src/package.rs`. The frozen fixtures in `tests/fixtures/v3/` pin these bytes: `crates/vpp-format/tests/fixtures_v3.rs` fails if the layout changes. The format came from the C++ implementation removed after commit `1d1cd11` (`git show 1d1cd11:runtime/src/package.cpp`).

A `.vpp` file is one page: its manifest, the table of its resources, the publisher's key and signature, and the resources themselves. A `.vppm` file is the same file cut off before the resource data, so the updater can check for changes with a few hundred bytes.

## Primitives

All integers are **little-endian**.

| Name | Encoding |
|---|---|
| `u16`, `u32`, `u64` | unsigned integer, 2, 4, or 8 bytes |
| `str` | `u32` length, then that many bytes. UTF-8 text, or raw bytes for keys and signatures. Readers refuse text fields that are not valid UTF-8; the C++ reader did not check. |
| `bytes[n]` | exactly `n` raw bytes |

Decoder limits: a `str` longer than 16 MiB (16 × 1024 × 1024 bytes), or a count above 1,048,576 (2²⁰), is an error. Every read checks that enough bytes remain; running past the end is an error, never a crash.

## Layout, version 3

```text
offset  field                   encoding        notes
──────  ──────────────────────  ──────────────  ─────────────────────────────────────────
0       magic                   bytes[4]        "VPPK"                                  ┐
4       version                 u16             3                                       │
6       site id                 str             from vpp.json "id"; required            │
        site name               str             from vpp.json "name"; defaults to id    │
        site version            str             from vpp.json "version"; default "0.0.0"│ signed
        page                    str             page name, e.g. "home"                  │ region
        window                  str             vpp.json "window" object as JSON text,  │
                                                or empty                                │
        resource count          u32             at most 2^20                            │
        per resource:                                                                   │
          name                  str             unique, non-empty; may contain "/"      │
          offset                u64             from the start of the data section      │
          size                  u64                                                     │
          sha256                bytes[32]       of the resource's bytes                 ┘
        publisher key           str             32 bytes (Ed25519), or empty
        signature               str             64 bytes (Ed25519), or empty
        data                    bytes           the resources, back to back
```

* **Resource order.** Writers store resources sorted by name (byte order), and assign offsets in that order with no gaps, so the same inputs always give the same bytes.
* **Resource names.** For example `dom.bin`, `style/00-global.bin`, `code/01-home.bin`. The compiler's numbering gives the load order.
* **Signed region.** Everything from the magic up to the end of the resource table. The signature is Ed25519 over exactly those bytes. Because the table holds every resource's hash, the signature covers the data too: changing any byte either breaks a hash or breaks the signature.
* **Key and signature.** Both present, and exactly 32 and 64 bytes, means signed. Both empty means unsigned. Anything else is a corrupt package.
* **`.vppm` manifest.** The bytes from the magic through the signature, with no data section. A `.vppm` with anything after the signature is corrupt.

## Checks a reader makes

1. Magic is `VPPK` and version is supported. Otherwise: not a package, or unsupported version.
2. Every resource name is non-empty and unique.
3. Key and signature are both present at the right sizes, or both empty.
4. **Full package only:** every resource's `offset + size` lies inside the data section.
5. **When reading a resource:** its SHA-256 matches the table. Otherwise the resource is refused.

The viewer then applies trust rules on top (`ARCHITECTURE.md`, invariants 1–3): signature valid for the embedded key, key matches the one pinned for the site id, version not older than the stored one.

## Version history

| Version | Change |
|---|---|
| 1 | First format. |
| 2 | Adds the `page` field. |
| 3 | Adds the `window` field (shell preferences from `vpp.json`, covered by the signature). |

Readers of version 3 accept only version 3; no earlier-version packages exist outside development. From the Rust port on, `docs/versioning.md` section 3 applies: readers accept a range, and every supported version has a fixture.

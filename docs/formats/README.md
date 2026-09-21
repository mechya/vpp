# Binary formats

One file per binary format, each giving the byte layout of every version still supported. The rules for changing a format are in `docs/versioning.md`, section 3.

| Format | Current version | Specification |
|---|---|---|
| `.vpp` page package | 3 | Currently in `packager/README.md`, "The .vpp format". It moves here as `package.md` when `vpp-format` is ported. |
| `.vppm` update manifest | — | To be written when `vpp-format` is ported |
| `dom.bin` | — | To be written when `vpp-dom` is ported |
| `style.bin` | — | To be written when `vpp-style` is ported |
| `code.bin` | — | To be written when `vpp-script` is ported |

The C++ implementation is the source of truth until each specification is written: `runtime/src/package.cpp` and `runtime/src/binary.cpp`.

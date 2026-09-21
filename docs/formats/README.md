# Binary formats

One file per binary format, each giving the byte layout of every version still supported. The rules for changing a format are in `docs/versioning.md`, section 3.

| Format | Current version | Specification |
|---|---|---|
| `.vpp` page package | 3 | [package.md](package.md) |
| `.vppm` update manifest | 3 | [package.md](package.md): the package without its data section |
| `dom.bin` | — | To be written when `vpp-dom` is ported |
| `style.bin` | — | To be written when `vpp-style` is ported |
| `code.bin` | — | To be written when `vpp-script` is ported |

Until a specification is written, the removed C++ implementation is the source of truth: `git show 1d1cd11:runtime/src/binary.cpp` for `dom.bin` and `style.bin`, and `docs/reference/compiler.md` for what they contain.

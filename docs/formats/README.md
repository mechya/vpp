# Binary formats

One file per binary format, each giving the byte layout of every version still supported. The rules for changing a format are in `docs/versioning.md`, section 3.

| Format | Current version | Specification |
|---|---|---|
| `.vpp` page package | 3 | [package.md](package.md) |
| `.vppm` update manifest | 3 | [package.md](package.md): the package without its data section |
| `dom.bin` | 1 | [dom.md](dom.md) |
| `style.bin` | 1 | [style.md](style.md) |
| `code.bin` | 1 | [code.md](code.md) |


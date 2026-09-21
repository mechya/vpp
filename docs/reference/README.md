# Reference specifications

How the C++ implementation of VPP behaved, kept as the specification for the Rust port. The C++ source was removed after commit `7cf865e`; read any file with `git show 7cf865e:<path>`, for example `git show 7cf865e:runtime/src/layout.cpp`.

| Page | Describes | Ported to |
|---|---|---|
| [compiler.md](compiler.md) | `vppc`: compiling a site, template rules, the binary resources | `vpp-compiler`, `vpp-template` |
| [packager.md](packager.md) | `vpppack`: keys, packaging, publishing, inspecting | `vpp-packager`, `vpp-format` |
| [updater.md](updater.md) | Hosting layout, update checks, the local store | `vpp-updater` |
| [viewer.md](viewer.md) | Modes, the shell, the `window` object, what the engine does | `vpp-viewer`, `vpp-desktop` |
| [runtime.md](runtime.md) | Supported CSS, the layout model | `vpp-dom`, `vpp-style`, `vpp-layout`, `vpp-paint`, `vpp-script` |

Byte layouts are in `docs/formats/`. When the Rust port deliberately changes a behaviour, `docs/rust-port.md` says so, and the page here gets a note.

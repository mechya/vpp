# Third-Party Notices

VPP is licensed under the VPP Source-Available License 1.0 (see `LICENSE.md`). The components below are separate works with their own licences, which continue to apply to them. They are fetched at build time by CMake and are not modified.

| Component | Used for | Licence | Source |
|---|---|---|---|
| SDL3 | window, input, presenting the pixel buffer | zlib | https://github.com/libsdl-org/SDL |
| stb_truetype | glyph rasterisation | public domain / MIT | https://github.com/nothings/stb |
| lexbor | HTML parsing in development mode and in the compiler | Apache 2.0 | https://github.com/lexbor/lexbor |
| quickjs-ng | JavaScript bytecode execution | MIT | https://github.com/quickjs-ng/quickjs |
| ed25519 (orlp) | publisher signatures | zlib | https://github.com/orlp/ed25519 |

The rendering engine (DOM, CSS, layout, painting, SVG paths), the binary formats, the package and update system, the template compiler, and the viewer shell are VPP's own code.

## Examples

`examples/app-window/assets/icons/` contains icons from Bootstrap Icons (https://icons.getbootstrap.com), MIT licensed, copyright The Bootstrap Authors. They are example content, not part of the VPP platform.

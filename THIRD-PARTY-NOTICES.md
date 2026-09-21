# Third-Party Notices

VPP is licensed under the VPP Source-Available License 1.0 (see `LICENSE.md`). Third-party components it uses are separate works under their own licences, which continue to apply to them.

## Rust dependencies

VPP's Rust crates are fetched by Cargo at build time and are not modified. Each direct dependency is added to the table below in the pull request that introduces it (`CONTRIBUTING.md`, "Dependencies"). Their own dependencies are checked by `cargo deny check` against the allowed licences in `deny.toml`.

| Component | Used for | Licence | Source |
|---|---|---|---|
| arboard | reading the clipboard, on Windows | MIT or Apache-2.0 | https://github.com/1Password/arboard |
| anyhow | error reporting in the tools | MIT or Apache-2.0 | https://github.com/dtolnay/anyhow |
| clap | command lines of the tools | MIT or Apache-2.0 | https://github.com/clap-rs/clap |
| cssparser | CSS tokenising and rule structure | MPL-2.0 | https://github.com/servo/rust-cssparser |
| ed25519-dalek | publisher signatures | BSD-3-Clause | https://github.com/dalek-cryptography/curve25519-dalek |
| getrandom | random seeds for new publisher keys | MIT or Apache-2.0 | https://github.com/rust-random/getrandom |
| html5ever | HTML parsing | MIT or Apache-2.0 | https://github.com/servo/html5ever |
| ring | cryptography for HTTPS | Apache-2.0 AND ISC | https://github.com/briansmith/ring |
| resvg, usvg, tiny-skia | painting shapes, and drawing SVG | MIT or Apache-2.0 (resvg, usvg); BSD-3-Clause (tiny-skia) | https://github.com/linebender/resvg, https://github.com/linebender/tiny-skia |
| rquickjs, with quickjs-ng | JavaScript: checking and running page scripts | MIT | https://github.com/DelSkayn/rquickjs, https://github.com/quickjs-ng/quickjs |
| serde, serde_json | reading `vpp.json` | MIT or Apache-2.0 | https://github.com/serde-rs |
| rustls | HTTPS | Apache-2.0 or ISC or MIT | https://github.com/rustls/rustls |
| rustls-platform-verifier | checking HTTPS certificates against the operating system's store | MIT or Apache-2.0 | https://github.com/rustls/rustls-platform-verifier |
| sha2 | SHA-256 resource hashes | MIT or Apache-2.0 | https://github.com/RustCrypto/hashes |
| softbuffer | showing the viewer's pixels in its window | MIT or Apache-2.0 | https://github.com/rust-windowing/softbuffer |
| slotmap | the DOM's node storage | Zlib | https://github.com/orlp/slotmap |
| swash | font loading and text measuring | MIT or Apache-2.0 | https://github.com/dfrg/swash |
| taffy | block and flex layout | MIT | https://github.com/DioxusLabs/taffy |
| thiserror | error types | MIT or Apache-2.0 | https://github.com/dtolnay/thiserror |
| ureq | HTTP client for updates | MIT or Apache-2.0 | https://github.com/algesten/ureq |
| winit | the desktop window, input, and the event loop | Apache-2.0 | https://github.com/rust-windowing/winit |

The C++ implementation removed after commit `7cf865e` used SDL3, stb_truetype, lexbor, quickjs-ng, and orlp's ed25519; the notices for them are in that commit.

`rustls-platform-verifier` also brings in `webpki-root-certs`, Mozilla's root certificate list, under the Community Data License Agreement – Permissive 2.0 (https://cdla.dev/permissive-2-0/).

`arboard` brings in `clipboard-win` and `error-code` on Windows, under the Boost Software License 1.0, which needs no notice in binaries.

`cssparser` is used unmodified. Its source, under the Mozilla Public License 2.0, is available at the address above.

## Test fonts

`tests/fixtures/fonts/` contains DejaVu Sans 2.37 for tests only; it is not shipped with VPP. Bitstream Vera Fonts licence, with DejaVu's changes in the public domain; see `tests/fixtures/fonts/LICENSE-DejaVu.txt`.

## Examples

`examples/app-window/assets/icons/` contains icons from Bootstrap Icons (https://icons.getbootstrap.com), MIT licensed, copyright The Bootstrap Authors. They are example content, not part of the VPP platform.

# Third-Party Notices

VPP is licensed under the VPP Source-Available License 1.0 (see `LICENSE.md`). Third-party components it uses are separate works under their own licences, which continue to apply to them.

## Rust dependencies

VPP's Rust crates are fetched by Cargo at build time and are not modified. None are used yet; each is added to the table below in the pull request that introduces it (`CONTRIBUTING.md`, "Dependencies"). `cargo deny check` enforces the allowed licences listed in `deny.toml`.

| Component | Used for | Licence | Source |
|---|---|---|---|

The C++ implementation removed after commit `1d1cd11` used SDL3, stb_truetype, lexbor, quickjs-ng, and orlp's ed25519; the notices for them are in that commit.

## Examples

`examples/app-window/assets/icons/` contains icons from Bootstrap Icons (https://icons.getbootstrap.com), MIT licensed, copyright The Bootstrap Authors. They are example content, not part of the VPP platform.

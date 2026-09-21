# Changelog

User-visible changes to VPP. Every pull request that changes something users will notice adds its line under **Unreleased** (see `docs/versioning.md`).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow [Semantic Versioning](https://semver.org); before 1.0, a minor version may contain breaking changes. The **Format** section lists binary format and `VPP.*` API level changes, so publishers can find them at a glance.

## Unreleased

### Added

- Rust workspace for the port from C++ (`docs/rust-port.md`): eleven crates under `crates/`, pinned toolchain, workspace lints, and CI on Windows, macOS, and Linux.
- Project files for contributors: `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CLA.md`, `GOVERNANCE.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `docs/versioning.md`.

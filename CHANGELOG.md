# Changelog

User-visible changes to VPP. Every pull request that changes something users will notice adds its line under **Unreleased** (see `docs/versioning.md`).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow [Semantic Versioning](https://semver.org); before 1.0, a minor version may contain breaking changes. The **Format** section lists binary format and `VPP.*` API level changes, so publishers can find them at a glance.

## Unreleased

### Added

- Rust workspace for the rewrite (`docs/rust-port.md`): fifteen crates under `crates/`, pinned toolchain, workspace lints, and CI on Windows.
- `platforms/` with a folder per operating system (Windows now; macOS, Linux, Android, iOS later), and `vpp-platform` with one source file per OS.
- `docs/formats/package.md`, the `.vpp` and `.vppm` byte layout (version 3), and `docs/reference/`, how the C++ tools behaved.
- Rules for AI-assisted contributions: `AGENTS.md` (read by AI coding tools; `CLAUDE.md` points to it), `CONTRIBUTING.md` section 10, `docs/review-checklist.md`, an AI section in the pull request template, and a CI check for hidden Unicode characters.
- Project files for contributors: `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CLA.md`, `GOVERNANCE.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `docs/versioning.md`.

### Removed

- The C++ implementation (viewer, runtime, compiler, packager, updater), `CMakeLists.txt`, and `build.cmd`. It remains in git history at commit `1d1cd11`. The tools come back from the Rust crates as the port proceeds.

# Versioning

VPP has several things that change at different speeds. Each has its own version, and they never share a number. This page is the rule book; `docs/rust-port.md` §6 records how it was decided.

| What | Version form | Where it lives | Who reads it |
|---|---|---|---|
| Software: viewer, tools, crates | SemVer, e.g. `0.4.0` | `Cargo.toml` (workspace) | Users, packagers |
| Binary formats: `.vpp`, `.vppm`, `dom.bin`, `style.bin`, `code.bin` | A single integer per format | `vpp-format/src/version.rs` | Viewer, tools |
| Page / site content | SemVer, set by the publisher | `vpp.json` → manifest | Updater (rollback protection) |
| `VPP.*` JavaScript API | Integer API level | `vpp.json` `"apiLevel"` | Viewer, page scripts |
| Template syntax | Covered by the compiler version | — | Compiler |
| Minimum Rust version | e.g. `1.85` | `Cargo.toml` `rust-version`, checked in CI | Contributors |

## 1. Software versions

* **One version for the whole workspace**, set once with `version.workspace = true`. The crates are released together, so there is no matrix of compatible crate versions for anyone to track.
* **`0.x` until the package format is declared stable.** In `0.x`, a minor bump (`0.4` → `0.5`) may break things and a patch bump may not. `1.0.0` means the package format and `VPP.*` API level 1 are frozen, apart from additive changes.
* **Releases are git tags** (`v0.4.0`) made from `main`. CI builds the viewer and tools for the three platforms and attaches them to the GitHub release.
* **Decision: VPP crates are not published to crates.io.** Every crate sets `publish = false`. The viewer and tools are distributed only as GitHub release binaries and through the source repository. Reasons:
  * A crates.io upload is permanent: a version can be yanked but never deleted.
  * Published crates spread through other projects' dependency trees, where the commercial-use threshold cannot be tracked or enforced.
  * It widens liability exposure beyond people who chose to download VPP.
  * The source-available licence is still a draft pending legal review.

  Revisiting this needs a design document (`CONTRIBUTING.md`, section 6) and the lawyer's approval of the licence for crates.io distribution.

## 2. Page and site versions

The publisher sets a SemVer version in `vpp.json`. The updater refuses any version lower than the one it has stored, which is the rollback protection. It is independent of the viewer version: a site at `3.2.0` runs in viewer `0.4.0`.

## 3. Binary format versions

* **Every binary format carries its own integer version** after its magic bytes. The package format is at version 3 today.
* **Any change to the byte layout bumps it.** No exceptions for "small" changes, because an old viewer must fail clearly rather than misread a file.
* **All constants live in `vpp-format/src/version.rs`** with a one-line history comment per version, as `package.cpp` does today (`// 2: page name; 3: window preferences`).
* **Readers accept a range, writers write one version.** The viewer reads from the oldest supported to the current version, and the tools always write the current one. Dropping support for an old version is a breaking change and is listed in `CHANGELOG.md`.
* **Every supported version has a fixture** in `tests/fixtures/`, and CI checks that each one still loads. A format bump without a new fixture fails review.
* **Each format has a specification** in `docs/formats/<name>.md` with the byte layout per version. `packager/README.md` currently holds this for packages; it moves there.
* **`code.bin` records which engine produced it**, e.g. `quickjs-ng 0.16`, because bytecode is tied to the engine build. On a mismatch the viewer refuses the page with a clear message rather than crashing inside the engine.
* **During the port**, the Rust code must read and write format version 3 byte for byte as the C++ tools do; that is how the port is proven correct. The first format change after the C++ tree is removed becomes version 4.

## 4. JavaScript API level

* A site declares the API level it was written for: `"apiLevel": 1` in `vpp.json`, recorded in each package.
* **Adding** a method or property does not raise the level. Pages feature-detect: `if (VPP.window.popup) …`.
* **Changing or removing** something raises the level. The viewer keeps serving the old behaviour to pages that declare the old level, for at least one minor release, and prints a deprecation warning in development mode.
* The viewer refuses a page that asks for a higher level than it supports, and tells the user to update the viewer.

## 5. Template syntax

Templates never reach the viewer, since the compiler dissolves them, so they need no runtime version. A breaking syntax change is a breaking compiler release under section 1, with a migration note in `CHANGELOG.md`.

## 6. Minimum Rust version

Two different settings:

* **Minimum supported Rust** is `rust-version` in `Cargo.toml` (currently 1.85, the first release with edition 2024). CI builds with it on every pull request. It is raised only in a minor release, with a changelog entry, and never to a Rust release less than about six months old, so contributors on distribution-packaged Rust are not locked out.
* **Development toolchain** is pinned in `rust-toolchain.toml`, so everyone formats and lints with the same Rust. It can be updated in any pull request, as long as the minimum still builds.

## 7. Changelog

* `CHANGELOG.md` in the [Keep a Changelog](https://keepachangelog.com) format, with an `Unreleased` section at the top.
* **Every pull request with a user-visible change adds its line** under `Unreleased`, in the same pull request. Releasing is then just renaming that heading.
* Sections: Added, Changed, Deprecated, Removed, Fixed, Security, plus **Format**, for any binary format or API level change, so publishers can find those at a glance.
* Commit messages follow the existing history: a short imperative subject (`Add shell UI and SVG icon support`). Conventional Commits are not required, since they are one more thing for first-time contributors to get wrong.

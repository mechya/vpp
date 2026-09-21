# Contributing to VPP

Thank you for helping. This page covers the agreement you sign, how to build and test, how code is laid out, and what a pull request needs.

## 1. Contributor License Agreement

**Every contributor signs the VPP Contributor License Agreement ([CLA.md](CLA.md)) before their first pull request is merged.**

VPP is source-available and also sold under commercial licences. The CLA lets the lead maintainer include your contribution in those commercial licences. You keep your copyright, you are owed no payment, and your contribution stays available to everyone under the public VPP Source-Available License.

Signing takes one comment. On your first pull request, the CLA Assistant bot links the agreement; reply with the sentence it gives. It applies to all your later pull requests too. If you contribute as part of your job, your employer signs Part B of the CLA first.

> The CLA is a draft pending legal review. Until the review is done and the CLA check is live, outside pull requests are welcome for discussion but are not merged.

## 2. Build and test

Install Rust with [rustup](https://rustup.rs). The repository pins its Rust version in `rust-toolchain.toml`, and rustup installs it on the first build. On Windows, the Visual Studio C++ build tools are also needed.

```sh
cargo build --workspace          # build everything
cargo test --workspace           # run all tests
cargo test -p vpp-format         # test one crate
cargo fmt --all                  # format
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vpp-viewer          # run the viewer
```

CI runs `fmt`, `clippy`, and `test` on Windows, macOS, and Linux, builds with the minimum supported Rust (`rust-version` in `Cargo.toml`), and checks licences and advisories with [cargo-deny](https://github.com/EmbarkStudios/cargo-deny). To run that last check locally: `cargo install --locked cargo-deny`, then `cargo deny check`.

**During the port**, the C++ tree is the reference implementation, and the Rust code is tested against its output. Building it needs CMake and the Visual Studio build tools; see `build.cmd`.

## 3. Where to start

* Read [ARCHITECTURE.md](ARCHITECTURE.md): the pipeline, the crates, and the invariants that must never break.
* Issues labelled **`good first issue`** are small and self-contained.
* **The port itself is the easiest way in.** The tracking issue has one checkbox per row of the C++ → Rust file map in `docs/rust-port.md` §5.3. Pick an unclaimed row, comment to claim it, and port it with the C++ file beside you.
* To show a bug, add a page to `examples/` that reproduces it. CI compiles and packages every example.

Labels: `good first issue`, `help wanted`, `area/<crate>`, `needs-design`, `format-change`.

## 4. How code is laid out

These rules keep VPP readable for the next contributor. Reviews check them.

* **One concept per file, named after the concept.** `cascade.rs`, `specificity.rs`, `line_break.rs`. No `utils.rs`, `helpers.rs`, or `misc.rs`.
* **Aim for under 400 lines per file.** Past that, split by concept.
* **`lib.rs` only declares modules and re-exports the public API.** It reads as the crate's table of contents, and starts with the crate's doc comment.
* **Module style:** `style.rs` next to a `style/` folder. No `mod.rs` files.
* **Private by default.** Items are `pub(crate)` unless another crate needs them.
* **Names from the specifications.** Use the terms of the HTML, CSS, and SVG specifications (`ComputedStyle`, `Specificity`, `InlineFormattingContext`), so the next person can search the specification for them. No abbreviations beyond the standard ones (`Dom`, `Css`, `Url`).
* **Comments explain why, not what.** Where code follows a specification, link the section, for example `// https://drafts.csswg.org/css-cascade-4/#cascade-sort`.
* **No clever code in shared paths.** No macros where a function works. No trait objects except at the declared boundaries (`ScriptEngine`, the HTTP client for tests). Generics only where they remove real duplication.
* **Errors say where.** Compiler and template errors carry file, line, and column: `pages/home.html:12:5: <vpp-fill> has no matching slot "sidebar"`.
* **Tests beside the code.** Unit tests go in a `#[cfg(test)] mod tests` at the bottom of the file they test. Cross-crate tests go in the crate's `tests/` folder.
* **No `unsafe`.** It is forbidden in every crate by the workspace lints. The one planned exception is the JavaScript engine boundary in `vpp-script`, where each use needs a `// SAFETY:` comment explaining why it holds.
* **Dependencies only point down** the crate table in `ARCHITECTURE.md`.

## 5. Pull request checklist

* [ ] `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` pass.
* [ ] Tests are added or updated. Rendering changes include a screenshot test.
* [ ] `CHANGELOG.md` has a line under **Unreleased** if users will notice.
* [ ] A binary format or API level change bumps its version, adds a fixture, and updates `docs/formats/` (`docs/versioning.md`).
* [ ] A new dependency is justified in the description (section 7).
* [ ] One focused change. Pull requests are squash-merged, so the history stays one line per change.

**Commit messages** follow the existing history: a short imperative subject, such as `Add shell UI and SVG icon support`. Conventional Commits are not required.

## 6. When a design document comes first

Small changes go straight to a pull request. These need a short design document first, agreed in an issue before code is written:

* any binary format change
* a new `VPP.*` API, or an API level change
* a change to signing, trust, or update behaviour
* a new crate, or a new heavy dependency
* runtime templating (`{{ }}`, bindings)

Copy `docs/design/0000-template.md` to `docs/design/NNNN-short-title.md` and open an issue linking it. The lead maintainer accepts or declines it with reasons ([GOVERNANCE.md](GOVERNANCE.md)).

## 7. Dependencies

A new dependency needs:

* a permissive licence (MIT, Apache 2.0, zlib, BSD, ISC, or similar); `deny.toml` holds the list and CI enforces it
* active maintenance
* a reason in the pull request description: what it replaces, and why VPP should not write it

Add its version to `[workspace.dependencies]` in the root `Cargo.toml`, and update `THIRD-PARTY-NOTICES.md` in the same pull request.

## 8. Security and conduct

* Report vulnerabilities privately; see [SECURITY.md](SECURITY.md). Never in a public issue.
* Everyone in the project follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## 9. Questions

Open an issue. If you are unsure whether something needs a design document, or which crate code belongs in, ask there before writing it.

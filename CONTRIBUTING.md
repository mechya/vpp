# Contributing to VPP

Thank you for helping. This page covers the agreement you sign, how to build and test, how code is laid out, what a pull request needs, and how to use AI tools (section 10).

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
cargo run -p vpp-desktop         # run the desktop viewer
```

CI runs `fmt`, `clippy`, and `test` on Windows (the current focus; macOS and Linux join when those platforms start), builds with the minimum supported Rust (`rust-version` in `Cargo.toml`), and checks licences and advisories with [cargo-deny](https://github.com/EmbarkStudios/cargo-deny). To run that last check locally: `cargo install --locked cargo-deny`, then `cargo deny check`.

**During the port**, the specification is written down: byte layouts in `docs/formats/`, and how each tool behaved in `docs/reference/`. The C++ implementation they came from was removed after commit `1d1cd11`; read any file of it with `git show 1d1cd11:<path>`, for example `git show 1d1cd11:runtime/src/layout.cpp`.

## 3. Where to start

* Read [ARCHITECTURE.md](ARCHITECTURE.md): the pipeline, the crates, and the invariants that must never break.
* Using an AI tool? Read section 10 first.
* Issues labelled **`good first issue`** are small and self-contained.
* **The port itself is the easiest way in.** The tracking issue has one checkbox per row of the C++ → Rust file map in `docs/rust-port.md` §5.3. Pick an unclaimed row, comment to claim it, and port it with the C++ file (`git show 1d1cd11:<path>`) beside you.
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
* **No `unsafe`.** It is forbidden in every crate by the workspace lints. The planned exceptions are the JavaScript engine boundary in `vpp-script`, and calls into Android and Apple APIs in `vpp-platform`, `vpp-android`, and `vpp-ios`. Each use needs a `// SAFETY:` comment explaining why it holds.
* **OS-specific code stays in its place.** `#[cfg(target_os = …)]` appears only in `vpp-platform` and the entry-point crates, and non-Rust platform files only in `platforms/<os>/` (`ARCHITECTURE.md`, "Platforms").
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

## 10. Using AI tools

**AI-assisted contributions, including "vibe coding", are welcome.** Use Claude, Copilot, Codex, Cursor, or anything else. The rules below exist so maintainers can review your change properly and catch bugs or harmful code, whoever or whatever wrote it.

### Before you start

1. **Read [ARCHITECTURE.md](ARCHITECTURE.md) and sections 4 to 7 of this page yourself.** Your AI tool can't tell you whether its change breaks an invariant if you don't know the invariants.
2. **Point your tool at [AGENTS.md](AGENTS.md).** Most AI coding tools read it automatically (Claude Code through `CLAUDE.md`). If yours does not, paste it into the conversation. It tells the tool what to read, what it must ask you about first, and what it must never do.
3. **Pick a small, clear task:** one row of the port's file map, or one `good first issue`. Say in the issue that you are working on it.

### While you work

* **The AI must stop and ask you** before it adds a dependency, writes `unsafe` code, touches signing, trust, or updates, changes a format, or edits CI or project files (`AGENTS.md`, section 2). If you agree, say so in the pull request.
* **Keep it small.** About 400 changed lines or fewer, excluding tests. AI tools produce large diffs easily; large diffs are hard to review and get sent back.
* **Don't let it wander.** Reformatting or "tidying" unrelated files hides real changes and will be rejected.

### Before you open the pull request

* **Read every line of the diff, and be able to explain it.** You are the author, not the tool. Reviewers will ask why, and "the AI wrote it" is not an answer.
* **Run the checks yourself** (section 2) and include the result. Do not trust the tool's claim that tests pass.
* **Check the facts.** AI tools invent specification sections, crate APIs, and flags. Follow every link and reference in your change.
* **Fill in the AI section of the pull request template:** which tool, and which parts it wrote. Disclosure is not a mark against you; it tells reviewers where to look closely.

### What reviewers do

Every pull request, AI-assisted or not, is reviewed against [docs/review-checklist.md](docs/review-checklist.md), including its checks for malicious or sneaky changes. Maintainers may use AI tools to help review, but a human maintainer always approves, and only the lead maintainer merges.

A pull request that is large, untested, or not understood by its author is closed with a pointer to this section. You are welcome to try again with a smaller one.

### Licensing

The CLA (section 1) applies as usual: you confirm you have the right to submit the change. AI tools sometimes reproduce code from other projects. If you recognise code from elsewhere, say where it came from; it is only accepted if its licence is on the allow list in `deny.toml`.

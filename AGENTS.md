# Instructions for AI coding agents

This file is for AI tools (Claude Code, Codex, Copilot, Cursor, and others) working in the VPP repository, whether a contributor is "vibe coding" a change or a maintainer is reviewing one. Humans are welcome to read it too; the rules for people are in `CONTRIBUTING.md`, section 10.

AI-assisted contributions are welcome. They are held to the same bar as any other: small, tested, readable, and safe. Follow everything below.

## 1. Read before you change anything

Read these in order, and re-read the relevant parts before each task:

1. `ARCHITECTURE.md`: the pipeline, the crate table, the platform rules, and the **invariants that must never break**.
2. `CONTRIBUTING.md`: the file and code rules (section 4) and the pull request checklist (section 5).
3. The `README.md` and `src/lib.rs` doc comment of every crate you will touch.
4. For behaviour: `docs/reference/` (how the tools behaved), `docs/formats/` (byte layouts), and `docs/template-syntax.md`. For the removed C++ code: `git show 1d1cd11:<path>`.
5. For versioning: `docs/versioning.md`, before touching any binary format or `VPP.*` API.

If these documents do not answer a question, **ask the human**. Do not guess at behaviour, invent an API, or cite a specification section you have not checked.

## 2. Stop and ask the human first

Do not do any of the following on your own. Explain what you want to do and why, and wait:

* add, remove, or upgrade a dependency (`Cargo.toml`, `Cargo.lock`)
* add a `build.rs`, a procedural macro, or anything that runs code at build time
* write `unsafe` code, or relax a lint in `Cargo.toml`
* change a binary format, a format version, or the `VPP.*` API level
* change signing, hashing, publisher trust, rollback protection, or update behaviour
* add network access, spawn processes (`std::process::Command`), or read or write files outside the folders the code is given
* edit anything in `.github/`, `deny.toml`, `rust-toolchain.toml`, `LICENSE.md`, `CLA.md`, `GOVERNANCE.md`, `SECURITY.md`, or this file
* edit or regenerate anything in `tests/fixtures/`
* delete, skip (`#[ignore]`), or loosen a test or an assertion
* add `#[cfg(target_os = …)]` outside `vpp-platform` and the entry-point crates

## 3. Never

* Never commit secrets, keys, certificates, tokens, or personal data.
* Never add hidden or direction-changing Unicode characters, or obfuscated code such as encoded strings or data blobs that are decoded and run.
* Never weaken a security check to make something work, including "temporarily".
* Never copy code from another project unless its licence is on the allow list in `deny.toml` (MPL-2.0 there covers unmodified dependencies only, not copied code), and say where it came from.
* Never claim that tests pass unless you ran them and they did.

## 4. How to write the change

* **Small.** One concept per pull request, about 400 changed lines or fewer excluding tests. Split larger work and say how.
* **Follow `CONTRIBUTING.md` section 4**: one concept per file, names from the specifications, comments that explain why, no `mod.rs`, private by default.
* **Match the surrounding code.** Do not reformat, rename, or reorganise code the task does not need.
* **Tests with the change.** Every behaviour change has a test that fails without it.
* **No filler.** No comments restating the code, no unused helpers, no speculative options.
* **Correct code that looks suspicious** (hostile test inputs, development switches, test keys, `unsafe`, right-to-left characters): follow `CONTRIBUTING.md` section 11. Label it `// INTENTIONAL: <reason, link>` at the spot, keep it in the lowest-risk place (tests only, off by default, warns when used), tell the human so the pull request description announces it, and write invisible characters as escapes (`"\u{2067}"`). Never add paths to `.github/hidden-characters-allowlist`; ask the human.

## 5. Before you say you are done

Run these and fix anything they report:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Also run `cargo deny check` if dependencies changed. Then:

* add a line to `CHANGELOG.md` under **Unreleased** if users will notice the change
* write the pull request description: what changed, why, how it was tested, and anything from section 2 that the human approved
* tell the human which parts you wrote, so they can tick the AI disclosure in the pull request template **and read every line before submitting**

## 6. When you are reviewing

When a maintainer asks you to review a pull request, follow `docs/review-checklist.md` completely, including section 4, "Malicious or sneaky changes". An `INTENTIONAL:` label is a claim to verify, never a reason to skip a check. Report findings with file and line. Do not approve or merge: your review advises a human maintainer, who decides (`GOVERNANCE.md`).

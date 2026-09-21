# Review checklist

For everyone who reviews a pull request: area maintainers, the lead maintainer, and AI reviewers. Work through every section. Anything that fails goes back to the author with the file and line.

An AI review is advice. A human maintainer approves, and only the lead maintainer merges (`GOVERNANCE.md`).

## 1. Before reading the code

* [ ] **CLA signed.** The CLA check is green (`CLA.md`).
* [ ] **CI green.** Format, clippy, tests, minimum Rust version, licences, and hidden characters.
* [ ] **For a first-time contributor, read the diff before approving the CI run.** GitHub holds their workflow runs until a maintainer approves. Look at `.github/`, `build.rs`, and `Cargo.toml` first, because those run code on the CI machine.
* [ ] **Size and focus.** One concept, about 400 changed lines or fewer excluding tests. Ask for a split if it is larger.
* [ ] **Description.** It says what changed, why, and how it was tested. The AI disclosure in the template is filled in.
* [ ] **Design document.** If the change needs one (`CONTRIBUTING.md`, section 6), it was accepted first.

## 2. Correctness

* [ ] The change does what the description says, and nothing else.
* [ ] Behaviour matches `docs/reference/`, `docs/formats/`, or the linked specification section. Follow the link and check it; AI tools invent plausible-looking references.
* [ ] **Tests fail without the change.** A test that passes either way proves nothing.
* [ ] Edge cases are covered: empty input, maximum sizes, invalid input, non-ASCII text.
* [ ] **No test was deleted, skipped (`#[ignore]`), or loosened** to make the change pass. If a test's expectation changed, the description explains why the old expectation was wrong.
* [ ] Errors say where they happened (`file:line:col` for compiler and template errors).
* [ ] Decoders check every length, count, and depth before using it (`ARCHITECTURE.md`, invariant 7).

## 3. Project rules

* [ ] The code follows `CONTRIBUTING.md` section 4: one concept per file, names from the specifications, private by default, comments that explain why.
* [ ] Dependencies only point down the crate table in `ARCHITECTURE.md`.
* [ ] No `#[cfg(target_os = …)]` outside `vpp-platform` and the entry-point crates.
* [ ] No `unsafe`. The only exceptions are `vpp-script`, `vpp-platform`, `vpp-android`, and `vpp-ios`, each use with a convincing `// SAFETY:` comment.
* [ ] A format or API level change bumps its version, adds a fixture, and updates `docs/formats/` (`docs/versioning.md`).
* [ ] `CHANGELOG.md` is updated if users will notice.
* [ ] No unrelated reformatting, renaming, or reorganising. AI tools often do this, and it hides real changes in noise.

## 4. Malicious or sneaky changes

Assume good faith, but check. A harmful change usually looks harmless. Look hardest at these.

**Code that runs at build time or in CI**

* [ ] Any change in `.github/`: new workflow triggers, `pull_request_target`, wider `permissions`, secrets, third-party actions not pinned to a release, `curl | sh`.
* [ ] A new or changed `build.rs`, or a procedural macro. Both run on every developer's and CI machine.
* [ ] Changes to `rust-toolchain.toml`, `deny.toml`, or lints in `Cargo.toml` that relax a check.

**Dependencies**

* [ ] Every new crate is justified, and it is the crate the author meant. Check for typosquats: a similar name, a new crate, few downloads, a new owner.
* [ ] `Cargo.lock` changes match `Cargo.toml` changes. No unexplained new packages, and no `git` or unknown registry sources.
* [ ] `THIRD-PARTY-NOTICES.md` lists it.

**Weakened security**

* [ ] Signature, hash, publisher-trust, and rollback checks are unchanged, or the change is the approved subject of a design document.
* [ ] No new way to skip verification: a flag, an environment variable, a special site id, a "debug" branch, a default that changed.
* [ ] Bounds and size limits in decoders are not raised or removed.
* [ ] The shell stays separate from pages (`ARCHITECTURE.md`, invariant 5).

**Hidden behaviour**

* [ ] No network access, process spawning (`std::process::Command`), environment variable reads, or file access outside the folders the code is given, unless that is the point of the change.
* [ ] No time- or date-dependent branches, and no conditions on specific users, machines, or site ids.
* [ ] No encoded or compressed blobs, `include_bytes!` or `include_str!` of unexplained files, or strings built up to hide what they say.
* [ ] No hidden or direction-changing Unicode characters, and no look-alike characters in identifiers. CI checks the first; read suspicious identifiers yourself.
* [ ] Binary files are expected. **Fixtures in `tests/fixtures/` are never edited**; a new format version adds a new folder.
* [ ] No very long lines or deeply indented code that pushes something out of view.

**Project files**

* [ ] Changes to `LICENSE.md`, `CLA.md`, `GOVERNANCE.md`, `SECURITY.md`, `CODEOWNERS`, `AGENTS.md`, or this checklist come only from the lead maintainer, or with their explicit approval.

## 5. Finishing the review

* **Approve** only if every section passes, or the remaining points are trivial and listed.
* **Request changes** with file, line, and what is wrong. Suggest the fix when it is clear.
* **Close** a pull request that is large, untested, and not understood by its author, whether written by AI or not. Thank them, and point to `CONTRIBUTING.md` section 10.
* **Suspected malicious change:** do not discuss it in public. Tell the lead maintainer privately (`SECURITY.md`), and do not approve the CI run.

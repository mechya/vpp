# 0001 — The `code.bin` container

**Status:** Accepted by the lead maintainer, 2026-09-21: source in `code.bin`, minimum Rust raised to 1.87, ES modules later.
**Author:** Claude (AI), for @mechya
**Issue:** to be opened

## Problem

A page's scripts travel as `code/NN-<name>.bin` resources. The C++ compiler wrote raw QuickJS bytecode into them, with no header, and the viewer loaded that bytecode straight into the engine (`JS_ReadObject`). `docs/rust-port.md` §6.3 asks for a container that records which engine produced the bytecode, so a viewer with a different engine refuses it instead of misreading it. Designing that container brought up a bigger problem.

**Loading bytecode is not safe for a viewer that opens other people's pages.** QuickJS does not verify bytecode when it loads it: bytecode is trusted to have come from its own compiler, and malformed bytecode can corrupt the viewer's memory. VPP checks signatures, but a signature proves *who* published a page, not that the page is harmless. Anyone can create a publisher key. So with bytecode, **any publisher can ship a page that attacks the viewer of everyone who opens it**, which breaks the security model in `SECURITY.md` ("a page escaping its limits").

A browser avoids this by accepting only JavaScript *source* from the network. Its parser is built to handle hostile input, and the bytecode it makes never leaves the machine.

A second, practical problem: bytecode is tied to one exact engine build. Every viewer update that changes QuickJS would break every published page until its publisher recompiled it.

## Proposal

**`code.bin` carries JavaScript source, not bytecode.** The viewer compiles it when the page loads, as a browser does. The compiler checks every script for syntax errors at build time, so authors still find mistakes when they build, not when a user opens the page.

### Layout, version 1

```text
magic      bytes[4]   "VPPC"
version    u16        1
kind       u8         1 = classic script (the only kind for now; ES modules later)
filename   str        the source file name, e.g. "app.js", for error messages and stack traces
source     str        the JavaScript source, UTF-8
```

Primitives are those of `docs/formats/package.md`. A reader refuses any other magic, version, or kind, and anything after the source. The constants go in `vpp-format/src/version.rs` and the codec in `vpp-format/src/code_bin.rs`, because the container no longer depends on any engine. That is where the plan first put it.

### What changes

* **`vppc`** compiles each script with QuickJS to check its syntax, and reports errors as `file:line:col`. It then writes the container with the source. `vppc app.js` writes one container. `-g` no longer applies, because the viewer always keeps line numbers.
* **The viewer** (`vpp-script`) reads the container and gives the source to QuickJS, as it did in C++ development mode.
* **The package format does not change.** `code/*.bin` stays an opaque resource to `.vpp` version 3. The fixtures in `tests/fixtures/v3/` are unaffected.

## Alternatives considered

1. **Bytecode in a container with the engine's identity** (the plan's original idea):

   ```text
   "VPPC", u16 version, str engine (e.g. "quickjs-ng 0.10.1"), u8 flags (debug info kept), bytes bytecode
   ```

   This solves the version mismatch, but not the security problem: a hostile publisher still ships crafted bytecode. **Not recommended.**
2. **Bytecode, but only from trusted publishers.** The viewer would load bytecode only for an allow-listed set of publisher keys. VPP is meant to open anyone's pages, and an allow-list turns it into a gatekeeper that has to be run. Not recommended.
3. **Verify bytecode before loading it.** No verifier exists for QuickJS bytecode, and writing one means re-checking everything the engine's own compiler guarantees. Far too large and too easy to get wrong.
4. **Do nothing** (raw bytecode, as in C++): carries both problems above.

## Compatibility and trade-offs

* **Source becomes visible.** This is the real cost. `README.md` lists "JavaScript compilation into binary form" as a design goal, and `docs/reference/compiler.md` says a distributed page does not contain its source. With this proposal, anyone can read a page's JavaScript by unpacking it, as on the web. Those two documents would be updated to say so. The old guarantee was weak anyway: `compiler.md` already notes that bytecode "is not encryption, and a determined person can" read it back. A minifier could later make shipped source harder to read, if wanted; it would be a separate decision with its own dependency.
* **Pages survive viewer updates.** Source does not depend on the engine version, so no page needs recompiling when QuickJS is updated.
* **Load time.** Compiling at load costs some milliseconds per script, which is small for pages of this size. The viewer can cache compiled bytecode on the user's own machine later. That cache stays safe, because the viewer made the bytecode itself.
* **Security.** The engine's parser is the only part exposed to page authors, and a JavaScript parser is built for untrusted input. The pages then run under VPP's own limits (`VPP.*` API level and permissions).
* **Minimum Rust version.** The current `rquickjs` (0.14) needs Rust 1.87; VPP's minimum is 1.85. `docs/versioning.md` allows raising it in a minor release to a Rust at least about six months old. 1.87 is from May 2025, so it qualifies. The alternative is `rquickjs` 0.11, which supports 1.85 but is three releases behind. Recommended: raise the minimum to 1.87.
* **Engine choice.** Which QuickJS `rquickjs` bundles (Fabrice Bellard's original or the `quickjs-ng` fork the C++ used) is checked when the dependency is added. With source in `code.bin`, the choice no longer affects the file format.

## Decisions

1. Shipped JavaScript is readable source; `README.md` and `docs/reference/compiler.md` say so.
2. The minimum Rust version is raised to 1.87 for `rquickjs` 0.14.
3. ES modules are left for later, as `kind = 2`.

`rquickjs` 0.14 bundles quickjs-ng 0.16.2, the same engine and version as the C++ tools.

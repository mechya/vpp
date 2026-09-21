# Changelog

User-visible changes to VPP. Every pull request that changes something users will notice adds its line under **Unreleased** (see `docs/versioning.md`).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow [Semantic Versioning](https://semver.org); before 1.0, a minor version may contain breaking changes. The **Format** section lists binary format and `VPP.*` API level changes, so publishers can find them at a glance.

## Unreleased

### Added

- `vpppack` in Rust: `keygen`, packaging and signing a site, `--publish` with manifests and shared resources, and `--inspect`. Same command line as the C++ tool.
- `vppc` in Rust: compiles pages (layouts, includes, components, scoped component CSS) into `dom.bin` and `style/*.bin`, byte for byte as the C++ tool did, and scripts into `code/*.bin` after checking their syntax with QuickJS (errors as `file:line:col`).
- `vpp-dom` (HTML parsing with html5ever, `dom.bin`), `vpp-style` (CSS parsing with cssparser, `style.bin`, selector matching), and `vpp-template`.
- `vpp-viewer` on Windows: the desktop viewer is back, in Rust: a frameless window with the shell bar (back, forward, reload, address field, window buttons), Ctrl+L, typing and pasting an address, keyboard shortcuts, dragging and resizing, and the `vpp.json` window options (theme, hidden bar or address field, size, `"height": "auto"`, rounded corners).
- `vpp-viewer` as a library, without its window yet: opens page sources, `dist` folders, signed packages, and URLs, verifies packages before running anything, runs page scripts, and handles clicks, links, history, and popups. Tested headlessly against both examples.
- `vpp-script`: runs page scripts with QuickJS: `document.getElementById`, `id`, `tagName`, `textContent`, `addEventListener` with clicks bubbling to ancestors, `console.log`, and `VPP.window.popup`, `close`, `minimize`, and `maximize`.
- `vpp-paint`: pages render to pixels: backgrounds, borders, rounded corners, text drawn from font outlines, and inline SVG through `resvg`. Screenshot tests render every example page with the test fonts.
- `vpp-layout`: block and flex layout through `taffy`, with VPP's own inline formatting and line breaking; `vpp-paint` loads fonts with `swash` and measures text for it.
- `vpp-style`: the cascade and computed styles, with the same supported CSS as the C++ engine (`docs/reference/runtime.md`).
- `vpp-updater`: the page store, publisher trust (moved out of the viewer), update checks that download only changed resources, rollback refusal, and the installed copy when offline. HTTPS through `ureq` and `rustls`, trusting the operating system's certificates.
- `vpp-format`: reading, writing, signing, and verifying `.vpp` packages and `.vppm` manifests; `vpp.json`; publisher key files.
- Rust workspace for the rewrite (`docs/rust-port.md`): fifteen crates under `crates/`, pinned toolchain, workspace lints, and CI on Windows.
- `platforms/` with a folder per operating system (Windows now; macOS, Linux, Android, iOS later), and `vpp-platform` with one source file per OS.
- `docs/formats/package.md`, the `.vpp` and `.vppm` byte layout (version 3), and `docs/reference/`, how the C++ tools behaved.
- Rules for AI-assisted contributions: `AGENTS.md` (read by AI coding tools; `CLAUDE.md` points to it), `CONTRIBUTING.md` section 10, `docs/review-checklist.md`, an AI section in the pull request template, and a CI check for hidden Unicode characters.
- Rules for correct code that looks suspicious: `INTENTIONAL:` labels (`CONTRIBUTING.md` section 11), reviewer checks, and `.github/hidden-characters-allowlist` for data files that must contain right-to-left or zero-width characters.
- Project files for contributors: `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CLA.md`, `GOVERNANCE.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `docs/versioning.md`.

### Format

- `code.bin` version 1 (`docs/formats/code.md`): scripts ship as JavaScript source in a container, no longer as raw bytecode, so a hostile publisher cannot hand the viewer's engine crafted bytecode (`docs/design/0001-code-bin.md`). Packages built by the C++ compiler must be recompiled.
- `dom.bin` and `style.bin` version 1 are specified in `docs/formats/dom.md` and `docs/formats/style.md`. Readers now also refuse unknown combinator bytes and a stored specificity that does not match its selector.
- Package format version 3 (`.vpp`, `.vppm`) is implemented in Rust (`vpp-format`) and frozen by fixtures in `tests/fixtures/v3/`. Unchanged from the C++ tools, except that text fields must now be valid UTF-8.

### Changed

- `cornerRadius`: any value above 0 now asks Windows 11 for its own rounded corners, at the system's radius; Windows 10 keeps square corners. The Rust viewer's pixel output cannot be transparent on Windows.
- Only following a link into another site reveals the full shell. A site opened from the command line or typed into the address field starts the way it asks, as `docs/reference/viewer.md` always said.
- A release build of the viewer opens no console window.
- The viewer keeps its data (pinned publisher keys and installed sites) in `%LOCALAPPDATA%\VPP\Viewer`, not the roaming `%APPDATA%\VPP\Viewer`, since installed sites can be large. Keys pinned by the C++ viewer are not carried over.
- Minimum supported Rust raised from 1.85 to 1.87, for `rquickjs` 0.14.
- Shipped JavaScript is now readable source; `README.md` and `docs/reference/compiler.md` say so.

### Fixed

- Shell: a site's `theme` must be a colour; the C++ viewer pasted it into the shell's stylesheet, so a crafted theme could restyle or hide the address field. A long address is shortened from the left instead of pushing the window buttons out of the window. Window sizes from `vpp.json` are limited to 80–8000 CSS pixels.
- Viewer: an error page shows the error message as text; the C++ viewer parsed it as HTML. Links with another scheme, such as `mailto:` or `javascript:`, are ignored instead of opened as file paths.
- Scripts: each page's scripts are limited to 256 MiB of memory and 5 seconds per script or event, so an endless loop no longer freezes the viewer. A listener that throws is logged and the other listeners still run. Using an element after it was removed from the page throws a `TypeError`.
- Rounded borders keep an even width; inline SVG supports all of SVG instead of filled paths only.
- Layout: margins collapse between a parent and its first or last child, flex items shrink by the flexbox algorithm, and text directly inside a flex container is laid out instead of dropped.

- Templates: ordinary element nesting deeper than 32 levels no longer fails as "includes or components nest too deeply".
- Templates: an include at the top level of an included file is now expanded.
- Templates: `data:` URLs and `#fragment` references in `src` and `href` are no longer rewritten into broken project paths.

- Updater: store folder names are unique per site id and page name, and an id such as `..` can no longer point outside the store.

### Removed

- The C++ implementation (viewer, runtime, compiler, packager, updater), `CMakeLists.txt`, and `build.cmd`. It remains in git history at commit `7cf865e`. The tools come back from the Rust crates as the port proceeds.

# VPP — Viewer Package Platform

## Rust Port Plan

**Status:** In progress. Step 1 (repository scaffolding) is done, and the C++ implementation has been removed; see §8. **Windows desktop comes first**; macOS, Linux, Android, and iOS follow (§4.1).
**Replaces:** the C++17 / CMake implementation (about 6,200 lines), removed after commit `7cf865e`. Read any of it with `git show 7cf865e:<path>`; its behaviour is described in `docs/reference/`.
**Purpose:** Record how the Rust version is built, laid out, versioned, and opened to contributors, so these choices are made once and not re-argued in every pull request.

Sections 5 to 7 were drafts of `ARCHITECTURE.md`, `docs/versioning.md`, `CONTRIBUTING.md`, and `GOVERNANCE.md`, which now exist. Where they differ from this plan, those files win.

---

# 1. Why Rust

* The viewer parses untrusted input: packages, HTML, CSS, SVG, and responses from the network. Safe Rust removes the memory-safety bugs that come with that.
* `cargo` replaces CMake, `FetchContent`, and `build.cmd`. For contributors, building is `cargo build` on Windows, macOS and Linux alike.
* The Rust ecosystem already has maintained, permissively licensed crates for most of what VPP builds by hand. Contributors can then work on what is specific to VPP.

# 2. Guiding rule

**VPP owns what makes VPP different.** That means the package format, the signing and update model, the template compiler, the DOM, the style cascade, the shell, and the `VPP.*` APIs.

**Everything else uses a well-known crate.** That covers parsing standard formats, box layout algorithms, rasterisation, fonts, cryptography, HTTP, and windowing. A contributor who already knows `html5ever`, `taffy`, or `winit` can then help on day one.

# 3. Library decisions

| Area | Choice | Replaces | Why |
|---|---|---|---|
| Window, input | `winit` + `softbuffer` | SDL3 | Pure Rust with no C build. `winit` is the standard Rust windowing crate. `softbuffer` presents VPP's own pixel buffer, which matches the current design. Frameless windows and `drag_window()` are supported. |
| Painting | `tiny-skia` | `paint.cpp`, `canvas.cpp` | Anti-aliased paths, rounded rectangles, gradients, and clipping into a CPU pixel buffer. It also backs the canvas API. |
| SVG | `resvg` / `usvg` | `svg.cpp` | Full SVG support built on `tiny-skia`, maintained by the same author. VPP no longer has to grow its own SVG subset. |
| Fonts, text | `swash`, plus `rustybuzz` for shaping | `stb_truetype` | Handles hinting and colour glyphs, and supports shaping from the start. That matters for non-Latin scripts, which are hard to add later. |
| HTML parsing | `html5ever` | lexbor | Spec-compliant parser from Servo. Parsing only: the DOM stays VPP's. |
| CSS tokenising | `cssparser` | the tokeniser in `css.cpp` | Servo's CSS3 tokeniser. Property parsing, selector matching, and the cascade stay VPP's own (`vpp-style`). |
| Box layout | `taffy` | block and flex parts of `layout.cpp` | Block, flexbox, and CSS grid, maintained and used by Bevy, Dioxus, and Zed. VPP adds inline formatting and line breaking on top. |
| JavaScript | `rquickjs`, behind a `ScriptEngine` trait | quickjs-ng | Bundles quickjs-ng 0.16.2, the same engine the C++ tools used. The trait keeps a later move to `boa` (pure Rust) possible without touching the DOM bindings. |
| DOM storage | Arena with generational ids (`slotmap`) | pointer tree in `dom.cpp` | Parent, child, and sibling links are ids, not references. This avoids `Rc<RefCell<…>>`, makes JS handles plain integers, and turns stale handles into an error rather than a crash. |
| Signatures | `ed25519-dalek` | orlp ed25519 | Audited RustCrypto-family crate. |
| Hashing | `sha2` | `sha256.cpp` | Standard crate. |
| DOM storage arena | `slotmap` | — | Generational keys, as the DOM row above describes. Zlib licence. |
| Encryption (planned) | `chacha20poly1305` (XChaCha20-Poly1305) | nothing yet | For "encrypted application resources" once that is designed. Fast without AES hardware support, and nonce misuse is less likely with 192-bit nonces. |
| HTTP | `ureq` with `rustls` (and `ring`), verifying certificates with `rustls-platform-verifier` | `http.cpp` | Small and blocking, with no async runtime. Update checks run on a worker thread. No OpenSSL dependency. Certificates are checked against the operating system's store, as WinHTTP did, not a bundled list. |
| `vpp.json`, manifests | `serde` + `serde_json` | hand-written JSON reader in `package.cpp` | Nested objects such as `window` and future config without a custom parser. |
| Command line | `clap` (derive) | hand-written argument parsing | Consistent `--help` across the tools. |
| OS services on desktop | `directories` (data folders), `arboard` (clipboard) | `prefDir` and friends in `viewer/src/main.cpp` | Maintained, cross-platform, and small. Used only inside `vpp-platform` (§4.1). |
| OS services on Android | `jni`, with `android-activity` through `winit` | nothing yet | The standard way to reach Android APIs from Rust. Built with `cargo-ndk`. |
| OS services on Apple platforms | `objc2` and its framework crates | nothing yet | Maintained bindings to Foundation, AppKit, and UIKit, used where `winit` does not already cover a service. |
| Errors | `thiserror` in libraries, `anyhow` in binaries | `bool` + `std::string* error` | Typed errors that carry source locations, which the template spec requires for compiler errors. |

Every crate listed is under MIT, Apache 2.0, zlib, or a similarly permissive licence, except `cssparser`, which is MPL-2.0: allowed for unmodified dependencies since step 3 (`deny.toml`). `cargo-deny` enforces this (§7.7).

# 4. Workspace layout

```text
vpp/
├── Cargo.toml              workspace: members, shared version, dependency versions, lints
├── rust-toolchain.toml     pins the toolchain so every contributor builds the same way
├── deny.toml               allowed licences, banned crates, advisories
├── ARCHITECTURE.md         the map: read this first (§5.1)
├── CONTRIBUTING.md         how to build, test, and send a change (§7)
├── CLA.md                  contributor licence agreement (§7.1)
├── GOVERNANCE.md           who decides what (§7.3)
├── CHANGELOG.md            user-visible changes per release (§6.7)
├── SECURITY.md             how to report a vulnerability privately
├── crates/
│   ├── vpp-format/         byte reader/writer, format versions, package, manifest, keys, signing, hashing
│   ├── vpp-template/       layouts, includes, slots, components → expanded HTML
│   ├── vpp-dom/            arena DOM, HTML parsing into it, dom.bin
│   ├── vpp-style/          CSS parsing, style.bin, selectors, cascade, computed style
│   ├── vpp-layout/         taffy integration, inline formatting, line breaking
│   ├── vpp-paint/          display list → pixels: tiny-skia, swash, resvg
│   ├── vpp-script/         ScriptEngine trait, rquickjs backend, DOM and VPP.* bindings
│   ├── vpp-updater/        page store, publisher trust, manifest checks, downloads
│   ├── vpp-platform/       OS services: data folders, clipboard, open URL, system fonts (§4.1)
│   ├── vpp-compiler/       binary: pages → dom.bin, style.bin, code.bin
│   ├── vpp-packager/       binary: sign and publish a site folder
│   ├── vpp-viewer/         library: the whole viewer (shell, navigation, rendering loop, input)
│   ├── vpp-desktop/        binary `vpp-viewer`: entry point for Windows, macOS, and Linux
│   ├── vpp-android/        native library: entry point for the Android app
│   └── vpp-ios/            static library: entry point for the iOS app
├── platforms/              everything per OS that is not Rust (§4.1)
├── examples/               unchanged; also the integration test input
├── tests/fixtures/         packages of every supported format version (§6.3)
└── docs/
    ├── formats/            one file per binary format (§6.3)
    ├── reference/          how the removed C++ implementation behaved: the port's specification
    └── design/             design documents for larger changes (§7.5)
```

Why it is split this way:

* **Dependencies go one way.** `format` ← `dom` ← `style` ← `layout` ← `paint` ← `viewer`. No crate depends on one to its right. A contributor changing layout never has to understand the viewer.
* **Tools stay light.** The compiler and packager never pull in windowing or rasterisation, so they build fast and run in CI without a display.
* **One area per crate.** Each crate is something a contributor can own, and `CODEOWNERS` maps it to its reviewers.
* **`vpp-format` stands alone.** It has no heavy dependencies, so a package inspector or signature verifier can be built on it alone.

## 4.1 Platforms: Windows, macOS, Linux, Android, iOS

**Order: Windows desktop first.** macOS and Linux follow, then Android, then iOS (§8). The folders and placeholder files for all five exist from the start (`crates/vpp-platform/src/<os>.rs`, `platforms/<os>/`), so later work has a home and nothing needs restructuring.

Almost all of VPP is the same on every platform: formats, DOM, style, layout, paint, script, and most of the viewer. So there is **one codebase, not a copy per OS**. What differs lives in exactly two places, with one rule each:

* **Rust code that differs per OS lives only in `vpp-platform` and the three entry-point crates.** No other crate contains `#[cfg(target_os = …)]`.
* **Everything per OS that is not Rust lives in `platforms/<os>/`**: app projects, manifests, icons, installers, signing.

### `vpp-platform`: one API, one file per OS

```text
crates/vpp-platform/src/
├── lib.rs        the shared API; picks the OS file at compile time
├── windows.rs
├── macos.rs
├── linux.rs      X11 and Wayland
├── android.rs
└── ios.rs
```

`lib.rs` declares each file behind its target, for example `#[cfg(target_os = "android")] mod android;`, and re-exports the same functions from all of them. A contributor fixing an Android problem edits `android.rs` and nothing else.

What it provides:

| Service | Why the OS matters |
|---|---|
| Data, cache, and config folders | The page store, pinned publisher keys, and preferences go to a different place on each OS, and to the app's private folder on Android and iOS. |
| System font folders and fallback fonts | Text needs a font for every script, and each OS keeps them elsewhere. |
| Clipboard | For the address field and text in pages. |
| Open a URL in the system browser | For links that are not VPP pages. |
| Form factor: desktop or mobile | Chooses the shell (below). |
| Registering `.vpp` files and VPP links | File associations and URL schemes are set up differently on each OS. |

**Only `vpp-viewer` depends on `vpp-platform`.** The library crates take folders and services as arguments. For example, `vpp-updater` is given its store folder rather than asking the OS. That keeps them testable on any machine and free of OS code.

### The viewer is a library with thin entry points

Android and iOS cannot start a Rust program on its own; the phone's app project loads Rust as a library. So the viewer is split:

| Crate | Builds | Used by |
|---|---|---|
| `vpp-viewer` | Rust library: the whole viewer | the three crates below |
| `vpp-desktop` | the `vpp-viewer` executable | Windows, macOS, Linux |
| `vpp-android` | `libvpp_android.so` (a `cdylib`), with `android_main` | the Gradle project in `platforms/android/` |
| `vpp-ios` | `libvpp_ios.a` (a `staticlib`), with a C entry function | the Xcode project in `platforms/ios/` |

Each entry-point crate stays under about 100 lines: it creates the `winit` event loop and hands it to `vpp_viewer::run`. `winit` provides windows, input, and the event loop on all five platforms.

### Desktop and mobile shells

The shell is itself a VPP page, so there are two of them, chosen by the form factor from `vpp-platform`:

* **Desktop shell:** frameless window, back, forward, reload, address field, and the minimize, maximize, and close buttons, as today.
* **Mobile shell:** no window buttons. The address bar sits at the top and hides on scroll, and back follows the system: the Android back gesture and the iOS edge swipe. Touch input reaches pages as pointer events.

The invariants hold on both: pages cannot style or cover the shell, and moving to a different site reveals the real address.

### `platforms/`

```text
platforms/
├── assets/     the source icon (SVG); each platform's icons are generated from it
├── windows/    installer, .ico icon, application manifest
├── macos/      Info.plist, .icns icon, entitlements, signing and notarisation
├── linux/      .desktop file, icons, AppImage or Flatpak manifest
├── android/    Gradle project: AndroidManifest.xml, the activity, packaging
└── ios/        Xcode project: Info.plist, the Swift entry point, signing
```

Each folder has a `README.md` saying how to build and package for that OS, and which tools it needs, such as Android Studio and the NDK, or Xcode.

### Targets

| Platform | Status | Architectures | CI |
|---|---|---|---|
| Windows | **Now** | x86_64; aarch64 later | tests |
| macOS | Later (§8, step 8) | aarch64, x86_64 | tests, once started |
| Linux | Later (§8, step 8) | x86_64; aarch64 later | tests, once started |
| Android | Later (§8, step 9) | arm64-v8a, plus x86_64 for the emulator | build only, with `cargo-ndk` |
| iOS | Later, if allowed (§8, step 10) | aarch64 devices, aarch64 simulator | build only |

CI tests on Windows only for now. Each platform's CI job is added when its work starts.

### Risks to check before building mobile

* **The iOS App Store may reject VPP.** Apple limits apps that download and run code (App Store Review Guidelines 2.5.2) and requires apps that browse the web to use WebKit (2.5.6). VPP downloads pages and runs their JavaScript in its own engine. Read the guidelines current at the time, and decide whether iOS goes ahead, before any iOS work. Android has no equivalent rule.
* **`softbuffer` on mobile.** Confirm that it presents pixels on Android and iOS. If it does not, the fallback is presenting the same pixel buffer through `wgpu`, which changes only `vpp-viewer`'s presentation code.
* **The on-screen keyboard.** Typing in the address field and in page inputs needs the soft keyboard and text input on Android and iOS. `winit`'s support here is more limited than on desktop, so test it early.
* **QuickJS on mobile.** Confirm that `rquickjs` builds for the Android and iOS targets.

A **mobile spike** checks all four at the start of step 9 (§8): a window on Android and iOS that fills pixels from `softbuffer`, takes touch and keyboard input, and runs one line of JavaScript. Doing it after the Windows viewer is an accepted risk: if `softbuffer` fails on mobile, only the presentation code in `vpp-viewer` changes, not the rendering crates.

# 5. Source files that are easy to understand

## 5.1 Where a newcomer starts

* **`ARCHITECTURE.md`** at the root: one page with the pipeline diagram from `README.md`, the crate map above, and the invariants that must never break. For example: "a page never runs before its signature is verified" and "the shell and the page never share a stylesheet". It names files and types but does not link to line numbers, which go stale.
* **Crate-level doc comment (`//!`) at the top of every `lib.rs`:** what the crate does, where it sits in the pipeline, which file to read first, and what it deliberately does *not* do.
* **`README.md` in every crate:** the same summary for people browsing on GitHub, plus how to run that crate's tests alone.

## 5.2 File rules

* **One concept per file, named after the concept.** `cascade.rs`, `specificity.rs`, `line_break.rs`, not `utils.rs`, `helpers.rs`, or `misc.rs`.
* **Aim for under 400 lines per file.** Past that, split by concept. Today's `viewer/src/main.cpp` (963 lines) and `layout.cpp` (696) are the cases this rule exists for.
* **`lib.rs` only declares modules and re-exports the public API.** No logic lives there, so it reads as the crate's table of contents.
* **Modern module style:** `style.rs` next to a `style/` folder. No `mod.rs` files, so editor tabs aren't all named `mod.rs`.
* **Private by default.** Items are `pub(crate)` unless another crate needs them. The public API of each crate stays small enough to list in its `lib.rs`.
* **Names from the specifications.** Types and functions use the terms of the HTML, CSS, and SVG specifications (`ComputedStyle`, `Specificity`, `InlineFormattingContext`), so a contributor can search the specification for them. No abbreviations beyond the standard ones (`Dom`, `Css`, `Url`).
* **Comments explain why, not what.** Where code follows a specification, link the section: `// https://drafts.csswg.org/css-cascade-4/#cascade-sort`.
* **No clever code in shared paths.** No macros where a function works. No trait objects except at the declared boundaries (`ScriptEngine`, the HTTP client for tests). Generics only where they remove real duplication.
* **Errors say where.** Compiler and template errors carry file, line, and column. The same message format is used everywhere: `pages/home.html:12:5: <vpp-fill> has no matching slot "sidebar"`.
* **Tests beside the code.** Unit tests go in a `#[cfg(test)] mod tests` at the bottom of the file they test. Cross-crate tests go in the crate's `tests/` folder.

## 5.3 File map: C++ to Rust

This table is the checklist for the port. The tracking issue has one checkbox per row, and each row is a good first or second contribution. The C++ files are gone from the tree; read one with `git show 7cf865e:<path>`, and its behaviour in `docs/reference/`.

| Removed C++ file | Rust file(s) | Notes |
|---|---|---|
| `runtime/include/vpp/bytes.h` | `vpp-format/src/bytes.rs` | *Done.* Reader and writer primitives. |
| `runtime/src/binary.cpp` | `vpp-dom/src/binary.rs` (`dom.bin`), `vpp-style/src/binary.rs` (`style.bin`) | *Done.* Each codec lives next to the type it encodes, so `vpp-format` never depends on the DOM or CSS. Both use `vpp-format`'s byte reader and version numbers. |
| — | `vpp-format/src/code_bin.rs` | *Done.* The `code.bin` container: JavaScript source (§6.3, `docs/design/0001-code-bin.md`). |
| `runtime/src/script.cpp` (`ScriptHost::compile`) | `vpp-script/src/syntax.rs` | *Done.* The compile step became a syntax check; source is shipped instead of bytecode. |
| `runtime/src/package.cpp` | `vpp-format/src/package.rs`, `manifest.rs`, `site_config.rs` | *Done.* `.vpp` and `.vppm` in `package.rs`, the manifest fields in `manifest.rs`, `vpp.json` (serde) in `site_config.rs`. |
| `runtime/src/crypto.cpp` | `vpp-format/src/keys.rs`, `signature.rs`, `hex.rs` | *Done.* |
| `runtime/src/sha256.cpp` | `vpp-format/src/hash.rs` | *Done.* A thin wrapper over `sha2` that defines the `ResourceHash` type. |
| — | `vpp-format/src/version.rs` | *Done.* All format version constants in one place (§6.3). |
| `runtime/src/template.cpp` | `vpp-template/src/page.rs`, `expander.rs`, `layout.rs`, `include.rs`, `component.rs`, `slot.rs`, `substitute.rs`, `props.rs`, `scoped_css.rs`, `preprocess.rs`, `project.rs`, `error.rs` | *Done.* One file per feature of `docs/template-syntax.md`. Fixes three C++ bugs, listed in the crate's `lib.rs`. |
| `runtime/src/dom.cpp` | `vpp-dom/src/node.rs`, `document.rs`, `tree.rs`, `query.rs` | *Done.* `tree.rs` holds all arena operations: append, remove, traverse. |
| `runtime/src/html.cpp` | `vpp-dom/src/parse.rs` | *Done.* `html5ever` tree sink into the arena. The resource collection it also held moved to `vpp-template/src/page.rs`. |
| `runtime/src/css.cpp` | `vpp-style/src/parse.rs`, `selector.rs`, `stylesheet.rs` | *Done.* Specificity is a method in `selector.rs`, too small for its own file. |
| `runtime/src/style.cpp` | `vpp-style/src/cascade.rs`, `computed.rs`, `user_agent.rs`, `properties.rs`, `replaced.rs`, `values/{color,length,edges,number}.rs` | *Done* (step 5a). Adding a CSS property touches `properties.rs`, the property table, and a `values/` file if it needs a new kind of value (§7.6). |
| `runtime/src/layout.cpp` | `vpp-layout/src/layouter.rs`, `taffy_style.rs`, `inline.rs`, `line_break.rs`, `tree.rs`, `document.rs`, `measure.rs` | *Done* (step 5b). Block and flex through `taffy`; inline formatting and line breaking are VPP's. |
| `runtime/src/paint.cpp` | `vpp-paint/src/display_list.rs`, `raster.rs`, `text.rs` | *Done* (step 5c). The display list is the seam for a future GPU backend. |
| `runtime/src/font.cpp` | `vpp-paint/src/font.rs`, `text.rs` | *Done* (steps 5b and 5c). |
| `runtime/src/svg.cpp` | `vpp-paint/src/svg.rs` | *Done* (step 5c). Glue to `resvg`: full SVG instead of the C++ path subset. |
| `runtime/src/canvas.cpp` | `tiny-skia`'s `Pixmap` | *Done* (step 5c). The C++ `Canvas` was a plain pixel buffer, so no VPP file replaces it. The JavaScript canvas API, when it comes, gets its own file. |
| `runtime/src/script.cpp` | `vpp-script/src/engine.rs`, `quickjs.rs`, `bindings.rs`, `prelude.js` | *Done* (step 6). The page's API is written in JavaScript in `prelude.js` on a few checked natives in `bindings.rs`, so a new `VPP.*` API is a few lines in each rather than a new binding file. |
| `updater/src/http.cpp` | `vpp-updater/src/http.rs`, `fetch.rs` | *Done.* `fetch.rs` is the `Fetch` trait, so tests stand in for the network. |
| `updater/src/updater.cpp` | `vpp-updater/src/sync.rs`, `store.rs`, `version.rs` | *Done.* One `sync` does the check and the download, as in C++, so they are one file rather than `check.rs` and `download.rs`. |
| `viewer/src/main.cpp` (publisher trust) | `vpp-updater/src/trust.rs` | *Done.* Moved out of the viewer: security logic belongs in a tested library. |
| `viewer/src/main.cpp` (`prefDir`, folders) | `vpp-platform/src/{windows,macos,linux,android,ios}.rs` | *Done for Windows* (step 7b): the data folder and system fonts. One file per OS behind one API (§4.1). |
| `viewer/src/main.cpp` (the rest) | `vpp-viewer/src/lib.rs`, `app.rs`, `location.rs`, `page_loader.rs`, `page.rs`, `navigation.rs`, `render.rs`, `popup.rs` | *Done* (step 7a). Keyboard shortcuts are mapped in `vpp-desktop/src/window.rs` (7b). Follows the `// ---` sections already in the file. |
| `viewer/src/main.cpp` (`main`, arguments) | `vpp-desktop/src/main.rs`, `cli.rs`, `window.rs`, `present.rs` | *Done* (step 7b). The desktop entry point; `vpp-android` and `vpp-ios` are new. |
| `viewer/src/shell.cpp` | `vpp-viewer/src/shell.rs`, `window_prefs.rs`, `input.rs`; later `shell/mobile.rs` | *Done for the desktop* (step 7c). The mobile shell shares navigation and address handling when it comes (§4.1). |
| `compiler/src/main.cpp` | `vpp-compiler/src/main.rs`, `cli.rs`, `compile.rs` | *Done.* `main.rs` only parses arguments and reports errors. |
| `packager/src/main.cpp` | `vpp-packager/src/main.rs`, `cli.rs`, `publish.rs`, `keygen.rs`, `inspect.rs` | *Done.* |

# 6. Versioning

VPP has several things that change at different speeds. Each one has its own version, and they never share a number.

| What | Version form | Where it lives | Who reads it |
|---|---|---|---|
| Software: viewer, tools, crates | SemVer, e.g. `0.4.0` | `Cargo.toml` (workspace) | Users, packagers |
| Binary formats: `.vpp`, `.vppm`, `dom.bin`, `style.bin`, `code.bin` | A single integer per format | `vpp-format/src/version.rs` | Viewer, tools |
| Page / site content | SemVer, set by the publisher | `vpp.json` → manifest | Updater (rollback protection) |
| `VPP.*` JavaScript API | Integer API level | `vpp.json` `"apiLevel"` | Viewer, page scripts |
| Template syntax | Covered by the compiler version | — | Compiler |
| Minimum Rust version | e.g. `1.87` | `Cargo.toml` `rust-version` | Contributors |

## 6.1 Software versions

* **One version for the whole workspace**, set once with `version.workspace = true`. The crates are released together, so there is no matrix of compatible crate versions for anyone to track.
* **`0.x` until the package format is declared stable.** In `0.x`, a minor bump (`0.4` → `0.5`) may break things and a patch bump may not. `1.0.0` means the package format and `VPP.*` API level 1 are frozen, apart from additive changes.
* **Releases are git tags** (`v0.4.0`) made from `main`. CI builds the viewer and tools for each supported platform (Windows first) and attaches them to the GitHub release.
* **Decision: VPP crates are not published to crates.io.** Every crate sets `publish = false`. The viewer and tools are distributed only as GitHub release binaries and through the source repository. Reasons:
  * A crates.io upload is permanent: a version can be yanked but never deleted.
  * Published crates spread through other projects' dependency trees, where the commercial-use threshold cannot be tracked or enforced.
  * It widens liability exposure beyond people who chose to download VPP.
  * The source-available licence is still a draft pending legal review.

  Revisiting this needs a design document (§7.5) and the lawyer's approval of the licence for crates.io distribution.

## 6.2 Page and site versions

This is unchanged from today. The publisher sets a SemVer version in `vpp.json`. The updater refuses any version lower than the one it has stored, which is the rollback protection. It is independent of the viewer version: a site at `3.2.0` runs in viewer `0.4.0`.

## 6.3 Binary format versions

* **Every binary format carries its own integer version** after its magic bytes. The package format is at version 3 today.
* **Any change to the byte layout bumps it.** No exceptions for "small" changes, because an old viewer must fail clearly rather than misread a file.
* **All constants live in `vpp-format/src/version.rs`** with a one-line history comment per version, as `package.cpp` does today (`// 2: page name; 3: window preferences`).
* **Readers accept a range, writers write one version.** The viewer reads from the oldest supported to the current version, and the tools always write the current one. Dropping support for an old version is a breaking change and is listed in `CHANGELOG.md`.
* **Every supported version has a fixture** in `tests/fixtures/`, and CI checks that each one still loads. A format bump without a new fixture fails review.
* **Each format has a specification** in `docs/formats/<name>.md` with the byte layout per version. The package format's is `docs/formats/package.md`.
* **`code.bin` holds JavaScript source, not bytecode** (`docs/design/0001-code-bin.md`). Loading bytecode from a publisher would let a hostile one attack the engine, and bytecode is tied to one engine build. The viewer compiles the source itself.
* **During the port**, the Rust code implements format version 3 exactly as `docs/formats/package.md` specifies it, taken from the removed C++ implementation (commit `7cf865e`). There are no C++-built fixtures: the first fixtures are written by the Rust packager, checked against the specification, and then frozen. The first format change after that becomes version 4.

## 6.4 JavaScript API level

* A site declares the API level it was written for: `"apiLevel": 1` in `vpp.json`, recorded in each package.
* **Adding** a method or property does not raise the level. Pages feature-detect: `if (VPP.window.popup) …`.
* **Changing or removing** something raises the level. The viewer keeps serving the old behaviour to pages that declare the old level, for at least one minor release, and prints a deprecation warning in development mode.
* The viewer refuses a page that asks for a higher level than it supports, and tells the user to update the viewer.

## 6.5 Template syntax

Templates never reach the viewer, since the compiler dissolves them, so they need no runtime version. A breaking syntax change is a breaking compiler release under §6.1, with a migration note in `CHANGELOG.md`.

## 6.6 Minimum Rust version

This is pinned in `Cargo.toml` and `rust-toolchain.toml`. It is raised only in a minor release, with a changelog entry, and never to a version less than about six months old, so contributors on distribution-packaged Rust are not locked out.

## 6.7 Changelog

* `CHANGELOG.md` in the [Keep a Changelog](https://keepachangelog.com) format, with an `Unreleased` section at the top.
* **Every pull request with a user-visible change adds its line** under `Unreleased`, in the same pull request. Releasing is then just renaming that heading.
* Sections: Added, Changed, Deprecated, Removed, Fixed, Security, plus **Format**, for any binary format or API level change, so publishers can find those at a glance.
* Commit messages follow the existing history: a short imperative subject (`Add shell UI and SVG icon support`). Conventional Commits are not required, since they are one more thing for first-time contributors to get wrong.

# 7. Community contribution

## 7.1 Before the first outside pull request

**Decision: a licence-grant CLA, in place before the first outside pull request is merged.**

`LICENSE.md` §15 says contributions may be subject to a Contributor License Agreement. VPP is source-available and sells commercial licences, so the maintainer needs the right to commercially license every line in the official project.

* **Why not a DCO.** With a DCO, contributed code arrives under the VPP Source-Available License, the same terms any user gets. That does not include the right to sell commercial licences covering that code, so each contributor would have to be asked before every commercial sale, and code from anyone who refuses or cannot be reached would have to be rewritten. This does not scale past a handful of contributors.
* **Licence grant, not copyright assignment.** Contributors keep their copyright and grant the maintainer a perpetual, worldwide, irrevocable, royalty-free licence to use, modify, sublicense, and commercially license their contribution. This is enough for the commercial licence and meets less resistance than assignment.
* **Terms.** `CLA.md` is based on the Apache Individual CLA, with an explicit clause that contributions may be included in commercially licensed versions of VPP, and that the maintainer owes no payment, royalty, or revenue share. A corporate version covers people contributing as part of their job.
* **Fairness promise.** Every contribution stays available under the public source-available licence. This answers the most common objection to CLAs.
* **Collection.** CLA Assistant (GitHub app, or the CLA Assistant Lite action) comments on a contributor's first pull request. They sign by replying in the thread, and a required status check blocks the merge until they do.
* **Legal review.** `CLA.md` is reviewed by the same lawyer as `LICENSE.md` before it is used.

Until the CLA check is live, no outside code is merged. The policy goes at the top of `CONTRIBUTING.md`.

## 7.2 Files in the repository

| File | Contents |
|---|---|
| `CONTRIBUTING.md` | Contribution policy (§7.1), how to build and test, the file rules from §5.2, the pull request checklist, where to ask questions. |
| `CLA.md` | Individual and corporate Contributor License Agreement (§7.1). |
| `GOVERNANCE.md` | Who decides what, and how maintainers are chosen (§7.3). |
| `ARCHITECTURE.md` | The map (§5.1). |
| `AGENTS.md`, `CLAUDE.md` | Instructions that AI coding tools read before working: what to read, what to ask the human first, what never to do. |
| `docs/review-checklist.md` | What every reviewer checks, human or AI, including malicious or sneaky changes. |
| `CODE_OF_CONDUCT.md` | Contributor Covenant, with a contact address. |
| `SECURITY.md` | Private reporting through GitHub security advisories. The signing and update code makes this necessary from day one. |
| `.github/CODEOWNERS` | The lead maintainer owns `*`. Area maintainers are added per crate. `vpp-format`, `vpp-updater`, and anything touching signatures list the lead maintainer only. |
| `.github/ISSUE_TEMPLATE/` | Bug report (viewer version, OS, the `.vpp` if shareable), feature request, and "rendering differs from browsers" with an HTML/CSS snippet. |
| `.github/pull_request_template.md` | The checklist in §7.4. |

## 7.3 Governance

VPP has one lead maintainer, the licensor in `LICENSE.md`, who has the final say. Area maintainers share the review work, but only the lead maintainer changes `main`.

* **The lead maintainer decides** merges, releases, design documents (§7.5), format and API level changes, and anything about licensing or the CLA.
* **Area maintainers** are chosen by the lead maintainer and listed in `CODEOWNERS` for their crates. They review and approve pull requests in their area. They cannot merge.
* **Triage helpers** get GitHub's Triage role: they label, assign, and close issues, but cannot approve or merge.
* **Contributors** fork the repository and open pull requests.

A change flows: contributor opens a pull request → area maintainer approves → lead maintainer merges.

How GitHub enforces this:

* Area maintainers have Write access, because GitHub only counts approvals and code ownership from accounts with Write access.
* A ruleset on `main` turns on **Restrict updates** with only the lead maintainer on the bypass list, so nobody else can push or merge. It also requires a pull request, requires review from code owners, requires the CLA and CI checks, blocks force pushes, and restricts deletion.
* A tag ruleset lets only the lead maintainer create `v*` tags. Release signing keys stay with the lead maintainer.

This makes the lead maintainer the bottleneck for merges. If that becomes a problem, area maintainers can later be allowed to merge in their own crates, while `vpp-format`, `vpp-updater`, signing, and licensing stay with the lead maintainer.

## 7.4 Pull request checklist

* `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` pass locally. CI runs the same on Windows; macOS and Linux join when those platforms start.
* Tests are added or updated. Rendering changes include a screenshot test.
* `CHANGELOG.md` has a line under `Unreleased` if users will notice.
* A format or API level change bumps the version, adds a fixture, and updates `docs/formats/` (§6.3, §6.4).
* A new dependency is justified in the description (§7.7).
* One focused change per pull request. Pull requests are squash-merged, so the history stays one line per change.

## 7.5 When a design document is needed first

Small changes go straight to a pull request. These need a short design document in `docs/design/NNNN-title.md` first, agreed in an issue before code is written:

* any binary format change
* a new `VPP.*` API or an API level change
* a change to signing, trust, or update behaviour
* a new crate or a new heavy dependency
* runtime templating (`{{ }}`, bindings), already flagged in `docs/template-syntax.md`

A design document states the problem, the proposal, the alternatives considered, and the compatibility impact. `docs/template-syntax.md` is the model.

## 7.6 Making the first contribution easy

* **Labels:** `good first issue`, `help wanted`, `area/<crate>`, `needs-design`, `format-change`.
* **Step-by-step guides** in `docs/guides/` for the most common changes: *adding a CSS property*, *adding a `VPP.*` API method*, *adding a template feature*. Each lists the exact files to touch, following the file map in §5.3.
* **The port itself is the on-ramp.** One tracking issue, with one checkbox per row of §5.3. Most rows are self-contained, with the removed C++ file (`git show 7cf865e:<path>`) and its behaviour in `docs/reference/` to work from.
* **Examples double as tests.** A contributor can add a page to `examples/` that shows a bug. CI compiles and packages it, and screenshot tests catch regressions.

## 7.7 Code and dependency rules

* **Edition 2024**, with the toolchain pinned (§6.6).
* **`#![forbid(unsafe_code)]`** in every crate except `vpp-script` (the engine boundary) and `vpp-platform`, `vpp-android`, and `vpp-ios` (calls into Android and Apple APIs), which may need it. Any `unsafe` there needs a `// SAFETY:` comment explaining why it holds.
* **Workspace-wide lints** in the root `Cargo.toml`, so every crate has the same rules without repeating them.
* **CI:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and `cargo deny check` (licences and advisories), on Windows for now.
* **Fuzzing:** `cargo-fuzz` targets for package reading, the binary decoders, and the template compiler, run on a schedule.
* **New dependencies** need a permissive licence (MIT, Apache 2.0, zlib, BSD, or similar), active maintenance, and a reason. `THIRD-PARTY-NOTICES.md` is updated in the same pull request.

# 8. Port order

Each step ends with something testable. The C++ implementation was removed after step 1, so tests check the Rust code against the written specifications (`docs/formats/`, `docs/reference/`) instead of against C++ output.

1. **Repository scaffolding** *(done)*: workspace, toolchain pin, lints, CI, `deny.toml`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CLA.md`, `GOVERNANCE.md`, `CHANGELOG.md`, `SECURITY.md`, the crate skeletons, `platforms/`, and the removal of the C++ tree (last in commit `7cf865e`). Still to do on GitHub: the CLA Assistant check, the `main` rulesets, and the tracking issue. Contributors can join once the CLA check is live (§7.1).
2. **`vpp-format`** *(done)*: byte reader and writer, packages, manifests, keys, and `version.rs`. The `code.bin` container moved to `vpp-script`. Test: round trips, signing and verification, every check in `docs/formats/package.md`, and hostile inputs (truncated, oversized, duplicated names). The first fixtures are written here and frozen.
3. **Tools** *(done)*: `vpp-packager`; then, brought forward from step 5 because the compiler needs them, the DOM with HTML parsing and `dom.bin` (`vpp-dom`) and CSS parsing with `style.bin` (`vpp-style`); then `vpp-template` and `vpp-compiler`. Test: one unit test per template rule; both `examples/` compile, package, sign, and verify, and their `dom.bin` and `style.bin` are byte for byte what the C++ tools wrote (same SHA-256 as `docs/reference/updater.md` lists). Scripts are checked for syntax with QuickJS and shipped as source in `code.bin` (`docs/design/0001-code-bin.md`); the minimum Rust version rose to 1.87 for `rquickjs`.
4. **`vpp-updater`** *(done)*, including trust moved out of the viewer. Test: a local static server with two versions of a site; only changed resources are downloaded, rollback is refused, and a changed publisher key is refused.
5. **Rendering** *(done)*, in three checkpoints: 5a, the cascade and computed styles in `vpp-style` *(done)*; 5b, `vpp-layout` with `taffy` and text measuring with `swash` *(done)*; 5c, `vpp-paint` with `tiny-skia` and `resvg` *(done)*. Screenshot tests use DejaVu Sans, bundled for tests only, so they match on every machine. Test: screenshot tests that render the examples to PNG headlessly. Reference images are approved by hand the first time, and compared on every change after.
6. **`vpp-script`** *(done)*: running scripts with QuickJS through `rquickjs`, the `ScriptEngine` trait, DOM bindings, `console`, `VPP.window`. Test: the example scripts behave as `docs/reference/viewer.md` describes, and runaway or hostile scripts hit the memory and time limits.
7. **Windows desktop viewer** *(done)*, in three checkpoints: 7a, `vpp-viewer` as a library with no window: loading and verifying pages in every mode, running them, clicks, links, history, and the popup, tested headlessly *(done)*; 7b, the window: `vpp-desktop` with `winit` and `softbuffer`, `vpp-platform/src/windows.rs` (data folder, system fonts), mouse, keyboard shortcuts, and display scale, keeping the system title bar for now *(done)*; 7c, the shell bar, Ctrl+L and the address field, the `vpp.json` window options, frameless windows with dragging and rounded corners, and the clipboard (`arboard`) *(done)*. Test: both examples run by hand on Windows. `platforms/windows/` (installer, icon, file registration) comes with the release. Then tag `v0.1.0` and release for Windows.
8. **macOS and Linux:** `vpp-platform/src/macos.rs` and `linux.rs`, `platforms/macos/` and `platforms/linux/`, and their CI jobs. `vpp-desktop` and `vpp-viewer` are shared. Test: both examples run by hand on each.
9. **Android:** the mobile spike (§4.1) first, then `vpp-android`, `vpp-platform/src/android.rs`, the mobile shell, and `platforms/android/`. Test: both examples run on a phone and the emulator, including the back gesture and the keyboard.
10. **iOS**, only if the App Store check in §4.1 says it can ship: `vpp-ios`, `vpp-platform/src/ios.rs`, and `platforms/ios/`. Test: both examples run on a device and the simulator.

# 9. Not decided here

* GPU rendering (`wgpu`, `vello`). The CPU path through `tiny-skia` comes first. The display list in `vpp-paint` keeps a GPU backend possible later.
* Moving from `rquickjs` to `boa`. Revisit once `boa` performance and conformance are enough for real pages.
* The runtime templating design (`{{ }}`, bindings). See [template-syntax.md](template-syntax.md).
* Publishing crates to crates.io: decided against (§6.1).
* Minimum OS versions for each platform, and the installer format for each desktop OS (MSIX or MSI, DMG, AppImage or Flatpak). Decided in `platforms/<os>/README.md` when that platform is packaged.
* Whether iOS ships at all (§4.1).

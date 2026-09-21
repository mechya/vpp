# Architecture

Start here. This page is the map of VPP: what happens to a page from source to screen, which crate does each part, and the rules that must never break. It names files and types but not line numbers, which go stale.

> **The port is in progress.** VPP is being rewritten in Rust (`docs/rust-port.md`), Windows desktop first. The C++ implementation was removed after commit `1d1cd11`; `docs/reference/` describes how it behaved, and each crate's README names the C++ files it replaces. Until a crate is written, nothing in it runs yet.

## From source to screen

```text
  DEVELOPMENT                                   ON THE USER'S MACHINE

  pages/home.html                               home.vpp  (local file or URL)
  + layouts, includes, components                   │
  + CSS, JavaScript                                 ▼
        │                                       vpp-updater   store, publisher trust, updates
        ▼                                           │
  vpp-template   expand to one HTML page            ▼
        │                                       vpp-format    verify signature and hashes
        ▼                                           │
  vpp-dom        parse ──► dom.bin                  ▼
  vpp-style      parse ──► style.bin            vpp-dom / vpp-style   decode dom.bin, style.bin
  vpp-script     compile ─► code.bin                │
        │                                           ▼
        ▼                                       vpp-style     cascade → computed styles
  vpp-packager   hash, sign                         │
        │                                           ▼
        ▼                                       vpp-layout    boxes and lines
  home.vpp + home.vppm + res/                       │
  (any static server)                               ▼
                                                vpp-paint     display list → pixels
                                                    │
                                                    ▼
                                                vpp-viewer    window, shell, input, navigation
                                                    ▲
                                                vpp-script    runs code.bin; DOM changes
                                                              trigger style and layout again
```

`vppc` (`vpp-compiler`) drives the left column's first stages; `vpppack` (`vpp-packager`) the last one.

## Crates

| Crate | Does | Depends on |
|---|---|---|
| `vpp-format` | Byte reader and writer, every format version number, `.vpp` package, `.vppm` manifest, `code.bin` container, `vpp.json`, keys, signatures, hashes | — |
| `vpp-dom` | Arena DOM, HTML parsing, `dom.bin` | `vpp-format` |
| `vpp-style` | CSS parsing, `style.bin`, selectors, cascade, computed style | `vpp-format`, `vpp-dom` |
| `vpp-layout` | Block and flex (via `taffy`), inline formatting, line breaking; asks for text sizes through `TextMeasure` | `vpp-dom`, `vpp-style` |
| `vpp-paint` | Fonts (`swash`) and the real `TextMeasure`; display list, rasterising, text, SVG, canvas | `vpp-layout` |
| `vpp-script` | `ScriptEngine` trait, QuickJS backend, DOM and `VPP.*` bindings | `vpp-dom` |
| `vpp-template` | Layouts, includes, slots, components, scoped component CSS | `vpp-format`, `vpp-dom`, `vpp-style` |
| `vpp-updater` | Page store, publisher trust, manifest checks, downloads | `vpp-format` |
| `vpp-compiler` | `vppc` binary | format, template, dom, style, script |
| `vpp-packager` | `vpppack` binary | `vpp-format` |
| `vpp-platform` | OS services: data folders, system fonts, clipboard, open URL, form factor. One file per OS. | — |
| `vpp-viewer` | The whole viewer as a library: shell, navigation, rendering loop, input | every library crate |
| `vpp-desktop` | `vpp-viewer` executable for Windows now, macOS and Linux later | `vpp-viewer`, `vpp-platform` |
| `vpp-android` | Native library loaded by the Android app (later) | `vpp-viewer`, `vpp-platform` |
| `vpp-ios` | Static library linked by the iOS app (later) | `vpp-viewer`, `vpp-platform` |

**Dependencies only point down the table**, never up: `vpp-format` knows nothing of the DOM, and nothing depends on the entry points (`vpp-desktop`, `vpp-android`, `vpp-ios`). If a change seems to need an upward dependency, the code is in the wrong crate. Ask in the issue before working around it.

Each crate's `src/lib.rs` (or `src/main.rs`) starts with a doc comment saying what it does, where it sits, what it deliberately does not do, and which file to read first.

## Platforms

VPP targets Windows, macOS, Linux, Android, and iOS from one codebase, **Windows desktop first**. What differs per OS lives in exactly two places, which already exist for all five, so later platforms slot in without restructuring:

* **Rust code that differs per OS** lives only in `vpp-platform` (one file per OS: `windows.rs`, `macos.rs`, `linux.rs`, `android.rs`, `ios.rs`) and in the entry-point crates `vpp-desktop`, `vpp-android`, and `vpp-ios`. No other crate contains `#[cfg(target_os = …)]`.
* **Everything per OS that is not Rust** lives in `platforms/<os>/`: the Android Gradle project, the iOS Xcode project, installers, manifests, icons, and signing.

Only `vpp-viewer` and the entry points depend on `vpp-platform`. Library crates are given folders and services as arguments, so they stay testable on any machine. The viewer has a desktop shell and a mobile shell, chosen by the form factor `vpp-platform` reports. Details and risks: `docs/rust-port.md` §4.1.

## Invariants

These hold everywhere. A pull request that breaks one is not merged, however useful it is otherwise.

1. **Nothing from a package runs or renders before its signature and resource hashes are verified.** Verification happens in `vpp-format`, and trust decisions in `vpp-updater`, for local files and URLs alike. The only exceptions are for development and are explicit: opening a page's `.html` source directly, and the `--allow-unsigned` flag, which prints a warning for every unsigned package it accepts.
2. **A site's publisher key is pinned on first use.** A package for a known site id signed by a different key is refused.
3. **Older versions are refused.** The updater never replaces a stored page with a lower version.
4. **Resources are addressed by SHA-256.** A resource is identified by its hash, never by name alone, so shared resources are downloaded and stored once.
5. **The shell is a separate document.** Pages cannot style, script, or cover it. Ctrl+L always reveals the full shell with the real address, and moving to a different site always reveals it.
6. **Each page is its own program.** Navigating ends the current page's JavaScript. State that survives navigation goes through the storage API.
7. **Decoders never trust lengths or counts.** Every binary decoder checks bounds and depth, so a corrupt or hostile file fails with an error instead of a crash or a hang.
8. **Templates never reach the viewer.** The compiler resolves all of them; `dom.bin` is plain HTML structure.

## Where things are

| You want to… | Look in |
|---|---|
| Change a binary format | `vpp-format/src/version.rs` first, then `docs/formats/`, then the codec. See `docs/versioning.md`. |
| Add a CSS property | `vpp-style` (the `values/` folder and the property table) |
| Add a `VPP.*` API | `vpp-script/src/bindings/`, plus a design document (`CONTRIBUTING.md`) |
| Change what the viewer trusts | `vpp-updater`, plus a design document |
| Change the address bar or window buttons | `vpp-viewer/src/shell.rs`, then `shell/desktop.rs` or `shell/mobile.rs` |
| Fix something on one OS only | `vpp-platform/src/<os>.rs`, or `platforms/<os>/` if it is not Rust |
| Build or package for one OS | `platforms/<os>/README.md` |
| Understand the template syntax | `docs/template-syntax.md` |

## Further reading

* `README.md`: what VPP is for
* `docs/rust-port.md`: the port plan, library choices, and C++ → Rust file map
* `docs/reference/`: how the removed C++ implementation behaved, which the port reproduces
* `docs/formats/`: byte layouts of the binary formats
* `platforms/README.md`: what each OS folder holds
* `docs/versioning.md`: how software, formats, sites, and the API are versioned
* `CONTRIBUTING.md`: how to build, test, and send a change

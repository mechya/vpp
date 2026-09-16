# VPP Viewer — Viewer Package Platform

**VPP Viewer** is the user-facing application of **VPP — Viewer Package Platform**.

It provides the native window, web-viewing experience, navigation interface, and execution surface for normal websites and `.vpp` applications.

## Initial Goals

- Native frameless window
- Address/search bar
- Normal HTTPS navigation
- Search engine integration
- Address bar auto-hide
- `Ctrl + L` to reveal the address bar
- Back and forward navigation
- Reload
- Downloads
- VPP application detection

## Default UI

On startup:

```text
┌───────────────────────────────────────────────┐
│ Search or enter address                      │
├───────────────────────────────────────────────┤
│                                               │
│                                               │
└───────────────────────────────────────────────┘
```

After navigation:

```text
┌───────────────────────────────────────────────┐
│                                               │
│                Web Content                    │
│                                               │
└───────────────────────────────────────────────┘
```

The address bar is hidden by default after navigation.

## Future Features

- Multiple windows
- Multiple instances
- Tabs
- VPP app mode
- Permissions
- Developer tools
- Runtime process isolation
- Session restore

## Building

Requirements on Windows: Visual Studio 2019 or newer with the C++ workload. CMake and Ninja ship inside Visual Studio, and every other dependency is downloaded by CMake at configure time.

```text
build.cmd
```

Result:

```text
build\viewer\vpp_viewer.exe
```

On other platforms, once a toolchain is installed:

```text
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

## Current State

The viewer renders a page with its stylesheet and runs its JavaScript, all inside the VPP engine. It has two modes, chosen by what you pass it.

### Development mode

```text
build\viewer\vpp_viewer.exe examples\hello-world\pages\home.html
```

The page is expanded with its layout, includes, and components, then parsed as text along with every stylesheet and script it references. Links to other pages (`about.vpp`) open the matching page source. With no argument the viewer opens `examples/hello-world/pages/home.html` relative to the current directory. `R` or `F5` reloads everything, so you can edit and see the result immediately.

### Release mode

```text
build\compiler\vppc.exe examples\hello-world\index.html
build\packager\vpppack.exe examples\hello-world --key examples\hello-world\publisher.key
build\viewer\vpp_viewer.exe examples\hello-world\hello-world.vpp
```

Given a `.vpp` package, the viewer checks three things before running anything, and any failure shows an error page instead of the application:

1. The package carries a valid publisher signature. Unsigned packages are refused unless the viewer is started with `--allow-unsigned`, which is meant for development only.
2. The publisher key matches the one recorded for this application id. The first package seen for an id records its key in the viewer's preference directory under `trust/`. A later package for the same id signed by a different key is refused.
3. Every resource's SHA-256 matches its entry in the manifest.

Then it loads `dom.bin`, `style.bin`, and `code.bin` from the package, and the window title becomes the application name from the manifest.

The viewer also accepts a `dist` directory, or a path to its `dom.bin`, and loads the binaries straight from disk without hashes. That is handy while iterating on the compiler.

In both cases no HTML, CSS, or JavaScript text is parsed. The console reports the mode, the manifest, and what was verified.

### Remote mode

```text
build\viewer\vpp_viewer.exe https://example.com/hello/manifest.vppm
```

Given a URL, the viewer installs or updates the application into its local store and then runs it as a package. Only resources whose hashes are not already present are downloaded, older versions offered by the server are refused, and if the server cannot be reached the installed copy runs. `R` re-checks the server. See `updater/README.md` for the hosting layout and the checks.

### What the engine does

- HTML becomes VPP's own DOM, which is styled, laid out, and painted into a CPU pixel buffer.
- CSS comes from linked files, `<style>` blocks, and `style=""` attributes, in that cascade order. `runtime/README.md` lists the supported properties.
- Layout supports block flow with collapsing margins, inline text with wrapping and inline styling, inline-blocks, widths, max-width with auto margins, padding, borders, text-align, and flexbox rows and columns.
- Scripts see a small DOM API: `document.getElementById`, `addEventListener`, `textContent`, `id`, `tagName`, plus `console.log` and `VPP.window.popup` / `VPP.window.close`. Changing `textContent` re-runs layout.
- Drag the top strip to move the frameless window. `Esc` closes the popup, or the viewer.
- Layout runs in CSS pixels and paint applies the display scale, so the page looks the same size on a 100% and a 150% display.

### Third-party code in the pipeline

- lexbor parses HTML text into a tree in development mode. VPP copies that tree into its own DOM and discards it. Release mode does not use it.
- quickjs-ng executes JavaScript bytecode. The DOM bindings around it are VPP's.
- stb_truetype rasterises individual glyphs.
- SDL3 owns the window, input, and presents the pixel buffer.

The DOM, CSS parser, cascade, layout, painting, hit testing, binary formats, and the script bindings are VPP's own code in `runtime/`.

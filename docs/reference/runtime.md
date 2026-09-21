# VPP Runtime — Viewer Package Platform

> **Reference specification.** This page describes how the C++ runtime behaved before it was removed (last present in commit `1d1cd11`; read its source with `git show 1d1cd11:runtime/src/`). The Rust port in `crates/vpp-dom, vpp-style, vpp-layout, vpp-paint, and vpp-script` reproduces this behaviour unless `docs/rust-port.md` says otherwise. Commands use the tool names the Rust port keeps (`vppc`, `vpppack`, `vpp-viewer`); they work once that crate is ported.

**VPP Runtime** is the engine of **VPP — Viewer Package Platform**: the DOM, CSS, layout, painting, and the JavaScript bindings that VPP applications run on.

It was a static library, `vpp_runtime`, used by the viewer and the compiler. In Rust it is split into `vpp-dom`, `vpp-style`, `vpp-layout`, `vpp-paint`, and `vpp-script` (`ARCHITECTURE.md`).

## Modules

| Header | Role |
|---|---|
| `vpp/dom.h` | Document tree: elements, text, attributes, lookups |
| `vpp/html.h` | HTML text → DOM (lexbor does the parsing, the DOM is VPP's) |
| `vpp/css.h` | CSS text → rules; selector matching |
| `vpp/style.h` | User-agent defaults, cascade, inheritance → `ComputedStyle` |
| `vpp/layout.h` | DOM + styles → layout tree of positioned boxes and lines |
| `vpp/paint.h` | Layout tree → pixels on a `Canvas` |
| `vpp/canvas.h` | ARGB pixel buffer with rectangles, rounded rectangles, blending |
| `vpp/font.h` | TrueType loading and glyph rasterisation (stb_truetype) |
| `vpp/script.h` | JavaScript host: DOM bindings, events, source and bytecode execution |

## Supported CSS

Selectors: type, `.class`, `#id`, `*`, compound (`button.primary`), descendant (`.card p`), child (`ul > li`), and comma lists. Specificity and source order decide the cascade; `!important` and `style=""` attributes are honoured. Pseudo-classes, attribute selectors, and at-rules are skipped.

Properties:

- `display`: `none`, `block`, `inline`, `inline-block`, `flex`
- `color`, `background-color`, `background` (colour only)
- `font-size` (px, em, rem, %), `font-weight`
- `text-align`
- `margin`, `margin-*` (including `auto` left/right), `padding`, `padding-*`
- `border` (width and colour), `border-width`, `border-color`, `border-radius`
- `width`, `height`, `max-width`
- `flex-direction`, `justify-content`, `align-items`, `gap`, `flex-grow`, `flex`

Colours: named, `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `transparent`.

Inherited: `color`, `font-size`, `font-weight`, `text-align`.

## Layout model

- Block containers stack block-level children vertically. Adjacent vertical margins collapse.
- Inline content breaks into lines at the container width. Inline elements change style mid-line; inline-blocks and any element placed in inline content are laid out as shrink-to-fit boxes aligned on the baseline.
- Flex containers place element children along a row or column, with `gap`, `justify-content`, `align-items`, and `flex-grow` distribution. No wrapping yet.
- Widths are content-box: `width` plus padding plus border. `max-width` with `margin: 0 auto` centres a block.
- Layout works in CSS pixels; paint multiplies by the display scale.

- Inline `<svg>` elements are replaced boxes sized by their `width` and `height` attributes or CSS. Every `<path>` inside is filled by the engine's own vector rasteriser (`vpp/svg.h`): M, L, H, V, C, S, Q, T, A, and Z commands, nonzero or even-odd winding, antialiased. `fill="currentColor"` or no fill uses the CSS `color`. Enough for icon sets such as Bootstrap Icons.

Not yet: raster images, strokes and other SVG features, scrolling, `position`, floats, grid, flex wrapping, line-height, non-ASCII text.

## Security

Applications execute inside an isolated runtime. One application does not gain access to another VPP application, unrestricted filesystem locations, process memory, or protected operating-system resources. Runtime permissions are explicitly controlled by VPP.

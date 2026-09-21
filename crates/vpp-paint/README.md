# vpp-paint

Painting.

Builds a display list from laid-out boxes and rasterises it into a pixel buffer with `tiny-skia`, text with `swash`, and SVG with `resvg`. It also backs the canvas API.

**Where it sits:** The last stage before the window. The display list is the seam where a GPU backend could go later.

**Not in this crate:** Windows and input. The viewer owns the window and hands this crate a buffer to draw into.

**Depends on:** `vpp-layout`.

## Status

Implemented (`docs/rust-port.md` §8, steps 5b and 5c). It replaces the C++ `runtime/src/paint.cpp`, `font.cpp`, `svg.cpp`, and `canvas.cpp` (`git show 7cf865e:<path>`).

| File | Holds |
|---|---|
| `raster.rs` | `render_page` and `rasterize`: drawing with `tiny-skia`. Start here. |
| `display_list.rs` | `DisplayItem`: what a page draws, in order |
| `font.rs` | `Font`, `FontSet`: loading fonts and measuring text |
| `text.rs` | Drawing text from glyph outlines |
| `svg.rs` | Inline `<svg>` through `resvg` |

`tests/screenshots.rs` renders every example page with DejaVu Sans (`tests/fixtures/fonts/`) and compares it with the approved images in `tests/screenshots/`; see `CONTRIBUTING.md` for updating them.

## Test it on its own

```sh
cargo test -p vpp-paint
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

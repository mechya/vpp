# vpp-layout

Layout.

Turns styled elements into positioned boxes. Block, flex, and grid come from `taffy`; inline formatting and line breaking are VPP's own.

**Where it sits:** Between style and paint.

**Not in this crate:** Drawing. Layout produces boxes and text runs with positions; `vpp-paint` turns them into pixels.

**Depends on:** `vpp-dom`, `vpp-style`.

## Status

Implemented (`docs/rust-port.md` §8, step 5b). It replaces the C++ `runtime/src/layout.cpp` (`git show 7cf865e:<path>`). Differences are listed in `docs/reference/runtime.md`.

| File | Holds |
|---|---|
| `document.rs` | `layout_document`. Start here. |
| `layouter.rs` | Building the `taffy` tree, running it, reading the result back |
| `taffy_style.rs` | A computed style in `taffy`'s terms |
| `inline.rs` | Collecting inline content, and laying it out in lines |
| `line_break.rs` | Filling and aligning lines |
| `tree.rs` | `LayoutBox`, `Line`, `Fragment`: the result |
| `measure.rs` | `TextMeasure`, and `FixedMeasure` for tests |
| `geometry.rs` | `Rect` |

`tests/layout.rs` checks positions worked out by hand with `FixedMeasure`, and lays out every example page.

## Test it on its own

```sh
cargo test -p vpp-layout
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

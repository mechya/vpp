# vpp-style

CSS.

Parses stylesheets (tokenising with `cssparser`), encodes them as `style.bin`, matches selectors, runs the cascade, and produces a computed style for every element.

**Where it sits:** Between the DOM and layout. The compiler uses it to produce `style.bin`; the viewer uses it before every layout.

**Not in this crate:** Box sizes and positions. Those are `vpp-layout`'s job.

**Depends on:** `vpp-format`, `vpp-dom`.

## Status

Implemented (`docs/rust-port.md` §8, steps 3 and 5a). It replaces the C++ `runtime/src/css.cpp`, `style.cpp`, and the `style.bin` half of `binary.cpp` (`git show 7cf865e:<path>`).

| File | Holds |
|---|---|
| `stylesheet.rs` | `StyleSheet`, `Rule`, `Declaration`. Start here. |
| `selector.rs` | `Selector`, its specificity, and matching it against elements |
| `parse.rs` | CSS text to a `StyleSheet`, through `cssparser` |
| `binary.rs` | `style.bin` (`docs/formats/style.md`) |
| `cascade.rs` | `compute_style`: which declarations apply, in what order |
| `computed.rs` | `ComputedStyle` and its keyword types |
| `user_agent.rs` | Built-in default styles for HTML elements |
| `properties.rs` | The property table: what each declaration changes |
| `values/` | Colours, lengths, box edges, numbers |
| `replaced.rs` | The size of `<svg>` elements |

## Test it on its own

```sh
cargo test -p vpp-style
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

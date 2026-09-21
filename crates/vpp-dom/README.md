# vpp-dom

The document tree.

Holds a page as an arena of nodes linked by generational ids, parses HTML into it with `html5ever`, and encodes it as `dom.bin`.

**Where it sits:** Directly above `vpp-format`. Style, layout, script, and the template compiler all work on this tree.

**Not in this crate:** Styles, boxes, or pixels. Nodes know their tag, attributes, text, and relatives; nothing else.

**Depends on:** `vpp-format`.

## Status

Not ported yet. The C++ reference implementation is `runtime/src/dom.cpp`, `runtime/src/html.cpp`, and the `dom.bin` half of `runtime/src/binary.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.

## Test it on its own

```sh
cargo test -p vpp-dom
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

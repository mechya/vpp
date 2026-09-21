# vpp-dom

The document tree.

Holds a page as an arena of nodes linked by generational ids, parses HTML into it with `html5ever`, and encodes it as `dom.bin`.

**Where it sits:** Directly above `vpp-format`. Style, layout, script, and the template compiler all work on this tree.

**Not in this crate:** Styles, boxes, or pixels. Nodes know their tag, attributes, text, and relatives; nothing else.

**Depends on:** `vpp-format`.

## Status

Implemented (`docs/rust-port.md` §8, step 3). It replaces the C++ `runtime/src/dom.cpp`, `html.cpp`, and the `dom.bin` half of `binary.cpp` (`git show 7cf865e:<path>`).

| File | Holds |
|---|---|
| `document.rs` | `Document`: the arena of nodes and the root. Start here. |
| `node.rs` | `Node`, `NodeId`, `NodeData`, `Element`, `Attribute` |
| `tree.rs` | Every operation that changes the tree: append, insert, detach, remove, copy, traverse |
| `query.rs` | Finding nodes and reading text |
| `parse.rs` | HTML to a `Document`, through `html5ever` |
| `binary.rs` | `dom.bin` (`docs/formats/dom.md`) |

## Test it on its own

```sh
cargo test -p vpp-dom
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

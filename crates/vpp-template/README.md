# vpp-template

Build-time templates.

Expands layouts, includes, slots, and components (`docs/template-syntax.md`) into one plain HTML page, with errors that point at `file:line:col`.

**Where it sits:** Used only by the compiler. The viewer never sees a template.

**Not in this crate:** Runtime templating (`{{ }}`, bindings). That needs its own design document first.

**Depends on:** `vpp-dom`.

## Status

Implemented (`docs/rust-port.md` §8, step 3). It replaces the C++ `runtime/src/template.cpp` (`git show 1d1cd11:<path>`), and fixes three of its bugs, listed in `src/lib.rs`. The rules are in `docs/reference/compiler.md`, "Templates".

| File | Holds |
|---|---|
| `page.rs` | `expand_page`, and collecting a page's stylesheets and scripts. Start here. |
| `expander.rs` | Loading template files and walking a document to expand it |
| `layout.rs`, `include.rs`, `component.rs` | One file per template element |
| `slot.rs` | Moving content into slots |
| `substitute.rs`, `props.rs` | `{{ }}` values |
| `scoped_css.rs` | Scoping component styles |
| `preprocess.rs` | Text fixes before HTML parsing |
| `project.rs` | The project root, and resolving references safely inside it |

## Test it on its own

```sh
cargo test -p vpp-template
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

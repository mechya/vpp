# `dom.bin`: a finished page's tree

**Current version:** 1
**Implementation:** `crates/vpp-dom/src/binary.rs`. The layout came from the C++ implementation removed after commit `7cf865e` (`git show 7cf865e:runtime/src/binary.cpp`), and the Rust compiler reproduces its output byte for byte.

The compiler writes one `dom.bin` per page, with every template already expanded. The viewer loads it without an HTML parser. Primitives (`u8`, `u16`, `u32`, `str`) and decoder limits are the same as in [package.md](package.md).

## Layout, version 1

```text
magic       bytes[4]   "VPPD"
version     u16        1
children    list       the document's children
```

A **list** is a `u32` count, then that many nodes. A **node** is:

```text
kind        u8         1 = element, 2 = text

element:
  tag         str        lower-case tag name
  attributes  u32        count, then for each:
    name        str        lower-case
    value       str
  children    list

text:
  text        str        character data
```

## What the encoder does

* **Drops `<script>`, `<style>`, and `<link>` elements**, with their subtrees. Their content travels as separate `code/` and `style/` resources.
* **Collapses whitespace in text:** each run of ASCII whitespace becomes one space. Spaces at the start and end are kept, because they separate the text from the inline content next to it.
* Keeps elements and text only. Comments and doctypes never reach the tree (`crates/vpp-dom/src/parse.rs`).

## Checks a reader makes

1. Magic is `VPPD` and version is supported.
2. Node kinds are 1 or 2.
3. Nesting is at most 512 levels deep, so a hostile file cannot exhaust the stack.
4. Nothing follows the tree.

## Version history

| Version | Change |
|---|---|
| 1 | First format. |

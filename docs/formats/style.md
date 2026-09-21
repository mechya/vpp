# `style.bin`: one parsed stylesheet

**Current version:** 1
**Implementation:** `crates/vpp-style/src/binary.rs`. The layout came from the C++ implementation removed after commit `1d1cd11` (`git show 1d1cd11:runtime/src/binary.cpp`), and the Rust compiler reproduces its output byte for byte.

The compiler writes one `style/NN-<name>.bin` per stylesheet a page uses, in load order. The viewer loads them without a CSS parser; selector matching still happens at run time, because scripts can change the DOM. Primitives and decoder limits are the same as in [package.md](package.md).

## Layout, version 1

```text
magic         bytes[4]   "VPPS"
version       u16        1
rules         u32        count, then for each rule:
  selectors     u32        count, then for each selector:
    specificity   u32        ids × 10000 + classes × 100 + types (a non-negative i32)
    compounds     u32        count (at least 1), then for each:
      tag           str        lower-case tag name, or empty for any element
      id            str        or empty
      classes       u32        count, then each class as str
    combinators   u32        count, exactly one fewer than compounds, then each as u8:
                             0x20 (' ') descendant, 0x3E ('>') child
  declarations  u32        count, then for each:
    property      str        lower-case, except custom properties (--name)
    value         str        as written, trimmed, comments replaced by a space
    important     u8         1 if !important, else 0
```

`combinators[i]` joins `compounds[i]` and `compounds[i + 1]`, so `.card > p` is two compounds and one `>`.

## Checks a reader makes

1. Magic is `VPPS` and version is supported.
2. Every selector has at least one compound, and one fewer combinator than compounds.
3. Every combinator byte is `' '` or `'>'`. The C++ reader did not check this.
4. The stored specificity matches the selector's parts. The C++ reader did not check this.
5. Nothing follows the last rule.

## Version history

| Version | Change |
|---|---|
| 1 | First format. |

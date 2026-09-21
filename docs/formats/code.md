# `code.bin`: one script of a page

**Current version:** 1
**Implementation:** `crates/vpp-format/src/code_bin.rs`. **Design:** `docs/design/0001-code-bin.md`.

The compiler writes one `code/NN-<name>.bin` per script a page uses, in load order. It holds the script's **JavaScript source**, not bytecode: the viewer compiles it when the page loads, so a publisher can never hand the engine bytecode it did not make itself. The compiler checks every script for syntax errors before writing it. Primitives and decoder limits are the same as in [package.md](package.md).

## Layout, version 1

```text
magic      bytes[4]   "VPPC"
version    u16        1
kind       u8         1 = classic script; 2 is reserved for ES modules
filename   str        the source file name, e.g. "app.js", or "inline-1.js" for an inline script
source     str        the JavaScript source, UTF-8
```

## Checks a reader makes

1. Magic is `VPPC` and version is supported. Raw bytecode from the C++ compiler has no magic and is refused, with a message to recompile.
2. The kind is one this build knows.
3. Nothing follows the source.

## Version history

| Version | Change |
|---|---|
| 1 | First format: JavaScript source. The C++ tools wrote raw QuickJS bytecode with no header. |

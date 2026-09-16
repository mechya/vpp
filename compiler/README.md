# VPP Compiler — Viewer Package Platform

**VPP Compiler** is the source-code compilation component of **VPP — Viewer Package Platform**.

It converts development source code, initially JavaScript, into binary resources that can be executed by the VPP Runtime without distributing the original application source files.

## Initial Pipeline

```text
app.js
   ↓
Parse
   ↓
Compile
   ↓
Optimize
   ↓
code.bin
```

The distributed VPP application should not require the original JavaScript source.

## Development Builds

Development builds should retain enough information for:

- debugging
- source mapping
- breakpoints
- stack traces
- readable errors

## Release Builds

Release builds may:

- optimize code
- strip debug metadata
- generate binary resources
- encrypt resources
- generate hashes

Source maps or equivalent VPP debug maps should normally remain private.

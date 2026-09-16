# VPP Packager — Viewer Package Platform

**VPP Packager** is the application packaging component of **VPP — Viewer Package Platform**.

It combines compiled code, interface resources, assets, application metadata, security information, and other required resources into a distributable `.vpp` application package.

## Input

Example application project:

```text
weather/
├── vpp.json
├── index.html
├── style.css
├── app.js
└── assets/
```

## Output

```text
weather.vpp
```

The package may internally contain:

```text
manifest.bin
dom.bin
style.bin
code.bin
assets.bin
```

## Packaging Pipeline

```text
Application source
       ↓
Compiler
       ↓
Binary resources
       ↓
Packager
       ↓
Encryption
       ↓
Signing
       ↓
application.vpp
```

## Goals

- Small package size
- Fast loading
- Integrity verification
- Incremental-update compatibility
- Cross-platform compatibility

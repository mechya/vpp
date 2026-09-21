# VPP Tools — Viewer Package Platform

**VPP Tools** contains the development, debugging, inspection, building, and diagnostic utilities for **VPP — Viewer Package Platform**.

These tools support the complete VPP development workflow, from running applications in development mode to inspecting packages and preparing signed release builds.

## Planned Tools

- VPP Dev Viewer
- Debugger
- DOM inspector
- CSS inspector
- Console
- Network inspector
- Runtime inspector
- Package inspector
- Manifest inspector
- Signature verifier
- Resource hash verifier

## Development Command

A future development workflow may look like:

```text
vpp dev .
```

The application opens directly from its project directory with debugging enabled.

## Build Command

```text
vpp build .
```

Possible result:

```text
application.vpp
```

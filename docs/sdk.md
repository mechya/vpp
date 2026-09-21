# VPP SDK — Viewer Package Platform

**VPP SDK** defines the application-development APIs provided by **VPP — Viewer Package Platform**.

It gives VPP applications a consistent interface for interacting with windows, storage, files, networking, permissions, system features, updates, and other runtime services.

## Possible APIs

```text
VPP.window
VPP.storage
VPP.files
VPP.network
VPP.system
VPP.permissions
VPP.update
```

## Example

```javascript
VPP.window.close();
```

```javascript
const value = VPP.storage.get("username");
```

```javascript
VPP.storage.set("theme", "dark");
```

## Goals

- Stable API
- Secure permission model
- Consistent cross-platform behavior
- Backward compatibility
- Simple JavaScript usage

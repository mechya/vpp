# VPP Viewer — Viewer Package Platform

**VPP Viewer** is the user-facing application of **VPP — Viewer Package Platform**.

It provides the native window, web-viewing experience, navigation interface, and execution surface for normal websites and `.vpp` applications.

## Initial Goals

- Native frameless window
- Address/search bar
- Normal HTTPS navigation
- Search engine integration
- Address bar auto-hide
- `Ctrl + L` to reveal the address bar
- Back and forward navigation
- Reload
- Downloads
- VPP application detection

## Default UI

On startup:

```text
┌───────────────────────────────────────────────┐
│ Search or enter address                      │
├───────────────────────────────────────────────┤
│                                               │
│                                               │
└───────────────────────────────────────────────┘
```

After navigation:

```text
┌───────────────────────────────────────────────┐
│                                               │
│                Web Content                    │
│                                               │
└───────────────────────────────────────────────┘
```

The address bar is hidden by default after navigation.

## Future Features

- Multiple windows
- Multiple instances
- Tabs
- VPP app mode
- Permissions
- Developer tools
- Runtime process isolation
- Session restore

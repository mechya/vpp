# VPP Runtime — Viewer Package Platform

**VPP Runtime** is the application execution environment of **VPP — Viewer Package Platform**.

It runs compiled VPP application code and provides controlled access to the DOM, windows, storage, networking, events, permissions, and other platform APIs.

## Responsibilities

- Execute VPP JavaScript binaries
- Manage application lifecycle
- Dispatch events
- Provide DOM APIs
- Provide window APIs
- Provide storage APIs
- Provide network APIs
- Enforce permissions
- Isolate applications

## Runtime Flow

```text
code.bin
   ↓
VPP Runtime
   ↓
Application APIs
   ↓
DOM / Window / Storage / Network
```

## Security

Applications should execute inside an isolated runtime environment.

One application should not automatically gain access to:

- another VPP application
- unrestricted filesystem locations
- process memory
- protected operating-system resources

Runtime permissions will be explicitly controlled by VPP.

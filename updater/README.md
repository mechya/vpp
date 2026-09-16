# VPP Updater — Viewer Package Platform

**VPP Updater** is the secure update system of **VPP — Viewer Package Platform**.

It detects changed application resources, downloads only the files required for an update, verifies their integrity and publisher signatures, and safely installs the new application version.

## Example

Installed application:

```text
dom.bin       A100
style.bin     B200
code.bin      C300
assets.bin    D400
```

Remote version:

```text
dom.bin       A100
style.bin     B200
code.bin      F921
assets.bin    D400
```

Only:

```text
code.bin
```

needs to be downloaded.

## Verification

Before installing an update, VPP should verify:

1. Application ID
2. Publisher identity
3. Digital signature
4. Version
5. Resource hashes
6. Resource sizes
7. Rollback rules

## Goals

- Download only changed resources
- Support atomic updates
- Reject corrupted resources
- Reject unauthorized updates
- Prevent version rollback where required

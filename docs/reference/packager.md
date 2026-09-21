# VPP Packager — Viewer Package Platform

> **Reference specification.** This page describes how the C++ packager behaved before it was removed (last present in commit `1d1cd11`; read its source with `git show 1d1cd11:packager/src/main.cpp`). The Rust port in `crates/vpp-packager` reproduces this behaviour unless `docs/rust-port.md` says otherwise. Commands use the tool names the Rust port keeps (`vppc`, `vpppack`, `vpp-viewer`); they work once that crate is ported.

**VPP Packager** is the packaging and signing component of **VPP — Viewer Package Platform**.

It turns each compiled page of a site into a signed `.vpp` file, hashes every resource, and writes the hosting layout that lets any static server publish the site with each resource stored once.

## Current State

The packager is the `vpppack` tool, built into `vpppack`.

### Create a publisher key, once

```text
cd examples\hello-world
vpppack keygen
```

Writes `publisher.key` (the secret, keep it private and out of git) and `publisher.pub`. The tool refuses to overwrite an existing secret key. Both file types are in `.gitignore`.

### Package and sign a site

A site needs a `vpp.json` in its root:

```json
{
  "id": "org.vpp.examples.hello-world",
  "name": "Hello World",
  "version": "1.0.0",
  "components": { "stat-card": "/components/stat-card" }
}
```

`id` identifies the site, is shared by all its pages, and should never change. `name` is what users see. `version` is free-form. `components` is optional and read by the compiler. An optional `window` object sets the viewer shell's theme colour and whether the address bar or the whole bar is shown; see `docs/reference/viewer.md`. It travels inside every page's manifest and is covered by the signature.

```text
vppc examples\hello-world
vpppack examples\hello-world --key examples\hello-world\publisher.key --publish
```

The first command compiles every page into `dist\<page>\`. The second packages each page, signs them all with the same key, and writes:

```text
examples\hello-world\out\
├── home.vpp        one signed package per page
├── about.vpp
├── home.vppm       each page's manifest, for update checks
├── about.vppm
└── res\            every distinct resource once, named by SHA-256
    ├── 5aa470bc…   global stylesheet, referenced by both pages
    ├── e7385e99…   shared script, referenced by both pages
    └── ...
```

Serve that directory as it is. Without `--publish` only the `.vpp` files are written. Use `-o <dir>` to choose another output directory. Without `--key` the pages are unsigned, the viewer refuses them unless started with `--allow-unsigned`, and `--publish` is refused.

The viewer opens a page directly:

```text
vpp-viewer examples\hello-world\out\home.vpp
```

### Inspect a package or manifest

```text
vpppack --inspect examples\hello-world\out\home.vpp
vpppack --inspect examples\hello-world\out\home.vppm
```

Prints the site, page, name, version, signature status and publisher key, and each resource with its size and hash prefix. For a full package every resource is verified; a modified file reports `INVALID` or `FAILED` and exits non-zero.

## The .vpp format

See `docs/formats/package.md` for the byte layout (version 3).

## What the viewer checks

1. The signature is present and valid for the embedded publisher key. Otherwise the page is refused before anything runs.
2. The publisher key matches the one recorded for this site id. The first page seen for a site records its key; a later page for the same site signed by another key is refused. The record lives in the viewer's preference directory under `trust/`.
3. Every resource's SHA-256 matches its entry.

Only then is the page loaded.

## Planned

- Assets: images and fonts with a virtual path table.
- An icon and a page list in `vpp.json`, for the viewer's start page and for prefetching a whole site.
- Key rotation: a page signed by the old key that announces the new one.

# VPP Updater — Viewer Package Platform

> **Reference specification.** This page describes how the C++ updater behaved before it was removed (last present in commit `1d1cd11`; read its source with `git show 1d1cd11:updater/src/updater.cpp`). The Rust port in `crates/vpp-updater` reproduces this behaviour unless `docs/rust-port.md` says otherwise. Commands use the tool names the Rust port keeps (`vppc`, `vpppack`, `vpp-viewer`); they work once that crate is ported.

**VPP Updater** is the installation and update system of **VPP — Viewer Package Platform**.

It fetches pages from any static web server, verifies their signatures, downloads only the resources not already present, and installs each page atomically. Pages are fetched when first visited and kept until they change. If the server is unreachable, the installed copy runs.

## Current State

The updater is the `vpp-updater` library, used by the viewer. There is no separate tool.

### Publish

```text
vpppack examples\hello-world --key examples\hello-world\publisher.key --publish
```

Writes a hosting layout into `out\` that any static server can serve, Tomcat, nginx, a Go file server, S3, or a plain directory listing:

```text
out\
├── home.vpp         full page packages, for direct downloads
├── about.vpp
├── home.vppm        signed manifests: resource names and hashes
├── about.vppm
└── res\
    ├── 5aa470bc…    global stylesheet, one file, used by both pages
    ├── e7385e99…    shared script
    ├── 4ff77d32…    home's dom.bin
    └── ...
```

Resource files are content-addressed, so they are immutable and can be cached forever by any proxy or CDN. Publishing a new version uploads the new resource files and replaces the manifests. Old resources can stay online as long as old manifests do.

Only signed pages can be published. The viewer never installs an unsigned manifest from the network.

### Install and run

```text
vpp-viewer https://example.com/hello/home.vpp
```

`vpp://example.com/hello/home.vpp` is accepted as an alias for `https://`.

On every visit to a page by URL, whether typed or reached through a link, the viewer:

1. Fetches the page's `.vppm` manifest, a few hundred bytes. If the server has no manifest, it downloads the whole `.vpp` instead.
2. Verifies the signature. Unsigned or invalid manifests are refused.
3. Compares it with the installed copy of that page. A different publisher key is refused. An older version is refused, which is the rollback protection.
4. If the manifest is unchanged, runs the installed copy with no further requests.
5. Otherwise downloads only the resources whose hashes are not already in the site's store, verifying each, then assembles the package and runs it with the same checks as any local `.vpp`.

If the manifest cannot be fetched at all and the page is installed, the installed copy runs and the console says so. That is what makes visited pages work offline.

### The local store

```text
%APPDATA%\VPP\Viewer\apps\<site id>\
├── pages\
│   ├── home.vppm    the installed manifest
│   ├── home.vpp     the assembled package the viewer runs
│   ├── home.url     where it came from
│   └── about.…
└── res\<sha256>     resources, shared by every page and version of the site
```

Every file is written to a temporary name and renamed into place, and a page's manifest is replaced last. An interrupted download leaves the previous version intact and runnable. A resource used by several pages is downloaded once; a page that shares everything with an installed page downloads only its own `dom.bin`.

## Changes in the Rust port

- **Store folder names** are made unique for every site id and page name: lower-case letters, digits, `-`, and `.` are kept, and every other byte is written as `_` and two hex digits. The C++ updater turned them all into `_`, so different ids could share a folder, and an id of `..` could leave the store.
- **The store and trust folders** are chosen by the viewer and passed to the updater; publisher trust (`trust/`) now lives in the updater library instead of the viewer.
- **A damaged trust record** is an error, never silently replaced by a new pin.
- **Responses** are limited to 256 MiB.

## Networking

The C++ updater used WinHTTP with the system proxy settings and short timeouts, so an unreachable server failed fast and the installed copy took over. The Rust port uses `ureq` with `rustls` (`docs/rust-port.md` §3) and keeps the short timeouts.

## Planned

- Garbage collection of resources no longer referenced by any installed page
- Prefetching every page of a site from a page list in the manifest, for full offline use from the first visit
- Background update checks while a page runs, with an "update available" event for scripts
- Download progress in the viewer window instead of the console

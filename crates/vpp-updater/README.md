# vpp-updater

Page store and updates.

Keeps visited pages on disk, pins each site's publisher key on first use, checks manifests, and downloads only the resources whose hashes changed. It refuses older versions.

**Where it sits:** Between the network and the viewer. Every page the viewer opens from a URL comes through here.

**Not in this crate:** The byte formats themselves (`vpp-format`).

**Depends on:** `vpp-format`.

## Status

Implemented (`docs/rust-port.md` §8, step 4). It replaces the C++ `updater/src/updater.cpp`, `http.cpp`, and the publisher-trust part of `viewer/src/main.cpp` (`git show 1d1cd11:<path>`). Differences are listed in `docs/reference/updater.md`.

| File | Holds |
|---|---|
| `sync.rs` | `Store::sync`: install and update a page. Start here. |
| `store.rs` | The layout on disk, safe folder names, atomic writes |
| `trust.rs` | Pinned publisher keys |
| `version.rs` | Version ordering, for rollback protection |
| `fetch.rs`, `http.rs` | The `Fetch` trait and the real HTTPS client |

`tests/static_server.rs` runs the updater against a real HTTP server on localhost.

## Test it on its own

```sh
cargo test -p vpp-updater
```

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for how this crate fits with the others, and [CONTRIBUTING.md](../../CONTRIBUTING.md) before sending a change.

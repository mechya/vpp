# Test fixtures

Packages, manifests, and binary resources of every supported format version. CI loads each one, so a change that breaks an older version fails the tests (`docs/versioning.md`, section 3).

* **Layout:** `v<package version>/<example>/`, for example `v3/hello-world/home.vpp`.
* **Origin:** the version 3 fixtures are produced by the C++ tools from `examples/`, and the Rust port must read them and write the same bytes (`docs/rust-port.md` §6.3).
* **Keys:** fixtures are signed with a test key kept in this folder. It is public and must never sign anything real.
* **Never edit a fixture by hand.** A new format version adds a new folder; old folders stay until that version's support is dropped.

These files are tracked in git on purpose, even though `*.vpp`, `*.bin`, and `*.key` are ignored elsewhere (see `.gitignore`).

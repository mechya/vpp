# Test fixtures

Packages, manifests, and binary resources of every supported format version. CI loads each one, so a change that breaks an older version fails the tests (`docs/versioning.md`, section 3).

* **Layout:** `v<package version>/`, then one folder per case: `v3/signed/home.vpp` and its `home.vppm`, `v3/unsigned/home.vpp`, and the test key `v3/test-publisher.key` and `.pub`.
* **Origin:** the version 3 fixtures were written once by `cargo run -p vpp-format --example write_fixtures`, which refuses to overwrite existing files. `crates/vpp-format/tests/fixtures_v3.rs` opens and verifies them, and rebuilds each package from the same inputs, requiring identical bytes.
* **Keys:** fixtures are signed with the published test key from RFC 8032 (section 7.1, test 1), kept in each version's folder. It is public on purpose and must never sign anything real.
* **Never edit a fixture by hand.** A new format version adds a new folder; old folders stay until that version's support is dropped.

These files are tracked in git on purpose, even though `*.vpp`, `*.bin`, and `*.key` are ignored elsewhere (see `.gitignore`). `.gitattributes` marks them binary, so git never converts their line endings.

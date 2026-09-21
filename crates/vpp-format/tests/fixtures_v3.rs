//! Package format version 3, frozen: the fixtures in `tests/fixtures/v3/` must
//! keep opening, verifying, and rebuilding byte for byte. If one of these tests
//! fails, the format changed; see `docs/versioning.md`, section 3.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use vpp_format::{Manifest, Package, PublicKey, SignatureStatus, SigningKey};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/v3")
        .join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// The same inputs as `examples/write_fixtures.rs`, written out again on purpose:
// this test checks that file, it does not share code with it.
fn manifest() -> Manifest {
    Manifest {
        site_id: "org.vpp.fixtures".into(),
        site_name: "VPP fixtures".into(),
        site_version: "1.0.0".into(),
        page: "home".into(),
        window: r##"{ "theme": "#2563eb", "addressBar": "hidden" }"##.into(),
    }
}

fn resources() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("dom.bin".into(), b"fixture dom.bin".to_vec()),
        (
            "style/00-global.bin".into(),
            b"fixture style/00-global.bin".to_vec(),
        ),
        (
            "code/00-app.bin".into(),
            b"fixture code/00-app.bin".to_vec(),
        ),
        ("code/01-empty.bin".into(), Vec::new()),
    ])
}

fn test_key() -> SigningKey {
    SigningKey::from_file_text(&String::from_utf8(fixture("test-publisher.key")).unwrap()).unwrap()
}

#[test]
fn test_key_is_the_published_rfc_8032_key() {
    let public =
        PublicKey::from_file_text(&String::from_utf8(fixture("test-publisher.pub")).unwrap())
            .unwrap();
    assert_eq!(test_key().public_key(), public);
    assert_eq!(
        public.to_string(),
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    );
}

#[test]
fn signed_package_opens_verifies_and_reads() {
    let package = Package::open(fixture("signed/home.vpp")).unwrap();
    assert_eq!(package.manifest(), &manifest());
    assert_eq!(package.verify_signature(), SignatureStatus::Valid);
    assert_eq!(package.publisher_key(), Some(&test_key().public_key()));
    for (name, bytes) in resources() {
        assert_eq!(package.read(&name).unwrap(), bytes.as_slice(), "{name}");
    }
}

#[test]
fn signed_package_rebuilds_byte_for_byte() {
    let rebuilt = Package::build(&manifest(), &resources(), Some(&test_key()));
    assert!(
        rebuilt == fixture("signed/home.vpp"),
        "Package::build no longer produces the frozen version 3 bytes"
    );
}

#[test]
fn unsigned_package_rebuilds_byte_for_byte() {
    let package = Package::open(fixture("unsigned/home.vpp")).unwrap();
    assert_eq!(package.verify_signature(), SignatureStatus::Unsigned);
    assert!(Package::build(&manifest(), &resources(), None) == fixture("unsigned/home.vpp"));
}

#[test]
fn manifest_matches_its_package_and_assembles_it() {
    let vppm = Package::open_manifest(fixture("signed/home.vppm")).unwrap();
    assert_eq!(vppm.verify_signature(), SignatureStatus::Valid);
    let full = Package::open(fixture("signed/home.vpp")).unwrap();
    assert_eq!(vppm.manifest_bytes(), full.manifest_bytes());
    assert!(Package::assemble(&vppm, &resources()).unwrap() == fixture("signed/home.vpp"));
}

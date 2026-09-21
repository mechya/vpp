//! Writes the package version 3 fixtures in `tests/fixtures/v3/`.
//!
//! Run once, from the repository root: `cargo run -p vpp-format --example write_fixtures`.
//! Fixtures are frozen: this refuses to overwrite a file that exists
//! (`tests/fixtures/README.md`). The integration test `tests/fixtures_v3.rs`
//! rebuilds each package from the same inputs and requires identical bytes.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use vpp_format::{Manifest, Package, SigningKey};

// INTENTIONAL: a published test key (RFC 8032, section 7.1, test 1), so anyone can
// check the fixtures' signatures. It must never sign anything real.
const TEST_SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];

const TEST_KEY_WARNING: &str = "# TEST KEY. Public on purpose: RFC 8032, section 7.1, test 1.\n# It signs only the fixtures in this folder. Never use it to sign anything real.\n";

fn main() -> std::io::Result<()> {
    let dir = Path::new("tests/fixtures/v3");
    fs::create_dir_all(dir.join("signed"))?;
    fs::create_dir_all(dir.join("unsigned"))?;

    let key = SigningKey::from_seed(TEST_SEED);
    let signed = Package::build(&manifest(), &resources(), Some(&key));
    let unsigned = Package::build(&manifest(), &resources(), None);
    let vppm = Package::open(signed.clone())
        .expect("a package just built opens")
        .manifest_bytes()
        .to_vec();

    write_new(
        &dir.join("test-publisher.key"),
        format!("{TEST_KEY_WARNING}{}", key.to_file_text()).as_bytes(),
    )?;
    write_new(
        &dir.join("test-publisher.pub"),
        key.public_key().to_file_text().as_bytes(),
    )?;
    write_new(&dir.join("signed/home.vpp"), &signed)?;
    write_new(&dir.join("signed/home.vppm"), &vppm)?;
    write_new(&dir.join("unsigned/home.vpp"), &unsigned)?;
    println!("wrote the version 3 fixtures in {}", dir.display());
    Ok(())
}

fn manifest() -> Manifest {
    Manifest {
        site_id: "org.vpp.fixtures".into(),
        site_name: "VPP fixtures".into(),
        site_version: "1.0.0".into(),
        page: "home".into(),
        window: r##"{ "theme": "#2563eb", "addressBar": "hidden" }"##.into(),
    }
}

/// Opaque bytes: these fixtures test the package container, not the resources inside it.
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

fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "{}: {e} (fixtures are frozen; see tests/fixtures/README.md)",
                    path.display()
                ),
            )
        })?;
    file.write_all(bytes)
}

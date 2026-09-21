//! `vpppack --inspect`: shows a package or manifest and verifies it.

use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use vpp_format::{Package, SignatureStatus};

/// Prints the manifest, signature status, and resources of `file`. For a full
/// package, every resource is checked against its hash. Returns whether
/// everything verified.
pub(crate) fn inspect(file: &Path, out: &mut dyn Write) -> Result<bool> {
    let bytes = fs::read(file).with_context(|| format!("cannot read {}", file.display()))?;
    let package = match Package::open(bytes.clone()) {
        Ok(package) => package,
        Err(full_error) => Package::open_manifest(bytes)
            .map_err(|_| full_error)
            .with_context(|| file.display().to_string())?,
    };

    let m = package.manifest();
    let kind = if package.has_data() {
        "package "
    } else {
        "manifest"
    };
    writeln!(out, "{kind}  {}", file.display())?;
    writeln!(out, "site     {}", m.site_id)?;
    writeln!(out, "page     {}", m.page)?;
    writeln!(out, "name     {}", m.site_name)?;
    writeln!(out, "version  {}", m.site_version)?;

    let mut ok = true;
    let publisher = package
        .publisher_key()
        .map(ToString::to_string)
        .unwrap_or_default();
    match package.verify_signature() {
        SignatureStatus::Unsigned => writeln!(out, "signed   no")?,
        SignatureStatus::Valid => writeln!(out, "signed   yes, valid\npublisher {publisher}")?,
        SignatureStatus::Invalid => {
            writeln!(
                out,
                "signed   yes, INVALID (manifest or hashes changed after signing)\npublisher {publisher}"
            )?;
            ok = false;
        }
    }

    writeln!(out, "resources")?;
    for entry in package.entries() {
        let status = if !package.has_data() {
            "listed"
        } else if package.read(&entry.name).is_ok() {
            "verified"
        } else {
            ok = false;
            "FAILED"
        };
        let hash = entry.hash.to_string();
        writeln!(
            out,
            "  {:<28} {:>8} bytes  {}  {status}",
            entry.name,
            entry.size,
            &hash[..16]
        )?;
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::test_dir::TestDir;
    use vpp_format::{Manifest, SigningKey};

    fn package(key: Option<&SigningKey>) -> Vec<u8> {
        let manifest = Manifest {
            site_id: "org.vpp.test".into(),
            page: "home".into(),
            ..Manifest::default()
        };
        let resources = BTreeMap::from([("dom.bin".to_string(), b"dom".to_vec())]);
        Package::build(&manifest, &resources, key)
    }

    fn inspect_bytes(bytes: &[u8]) -> (Result<bool>, String) {
        let dir = TestDir::new("inspect");
        dir.write("home.vpp", bytes);
        let mut out = Vec::new();
        let result = inspect(&dir.path().join("home.vpp"), &mut out);
        (result, String::from_utf8(out).unwrap())
    }

    #[test]
    fn verified_package() {
        let (result, log) = inspect_bytes(&package(Some(&SigningKey::from_seed([1; 32]))));
        assert!(result.unwrap());
        assert!(log.starts_with("package "));
        assert!(log.contains("signed   yes, valid"));
        assert!(log.contains("dom.bin"));
        assert!(log.contains("verified"));
    }

    #[test]
    fn manifest_lists_without_verifying_data() {
        let full = Package::open(package(None)).unwrap();
        let (result, log) = inspect_bytes(full.manifest_bytes());
        assert!(result.unwrap());
        assert!(log.starts_with("manifest"));
        assert!(log.contains("signed   no"));
        assert!(log.contains("listed"));
    }

    #[test]
    fn changed_resource_fails() {
        let mut bytes = package(Some(&SigningKey::from_seed([1; 32])));
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        let (result, log) = inspect_bytes(&bytes);
        assert!(!result.unwrap());
        assert!(log.contains("FAILED"));
    }

    #[test]
    fn not_a_package_is_an_error() {
        let (result, _) = inspect_bytes(b"hello");
        assert!(result.unwrap_err().to_string().contains("home.vpp"));
    }
}

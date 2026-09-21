//! The `.vpp` page package and its `.vppm` manifest (`docs/formats/package.md`).
//!
//! A package is a manifest, a table of resources with their SHA-256 hashes, an
//! optional publisher key and signature, and then the resources back to back.
//! The signature covers the manifest and the table; because the table holds
//! every hash, it covers the data too. A `.vppm` file is the same bytes cut off
//! before the data, so the updater can check for changes cheaply.
//!
//! Opening a package checks its structure. Whether to trust it is decided
//! elsewhere: [`Package::verify_signature`] says whether the signature is valid,
//! and `vpp-updater` decides whether that publisher is the right one.

use std::collections::{BTreeMap, HashSet};

use thiserror::Error;

use crate::bytes::{ByteReader, ByteWriter, DecodeError};
use crate::hash::ResourceHash;
use crate::keys::{PublicKey, SigningKey};
use crate::manifest::Manifest;
use crate::signature::Signature;
use crate::version::{PACKAGE_MAGIC, PACKAGE_VERSION};

/// Why a package could not be opened, read, or assembled.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PackageError {
    /// The bytes do not start with `VPPK`.
    #[error("not a .vpp package")]
    NotAPackage,
    /// The package uses a format version this build cannot read.
    #[error("unsupported package version {found}; this build reads version {supported}")]
    UnsupportedVersion {
        /// The version in the file.
        found: u16,
        /// The version this build reads.
        supported: u16,
    },
    /// The package's bytes do not decode.
    #[error("corrupt package: {0}")]
    Corrupt(#[from] DecodeError),
    /// A resource in the table has an empty name.
    #[error("corrupt package: a resource has an empty name")]
    EmptyResourceName,
    /// Two resources in the table have the same name.
    #[error("corrupt package: resource {0:?} is listed twice")]
    DuplicateResource(String),
    /// The publisher key and signature are not both present at the right sizes, nor both empty.
    #[error("corrupt package: the publisher key or signature has the wrong size")]
    MalformedSignature,
    /// A resource's offset and size point outside the data section.
    #[error("corrupt package: resource {0:?} lies outside the file")]
    OutOfBounds(String),
    /// A `.vppm` manifest has bytes after its signature.
    #[error("corrupt manifest: data after the signature")]
    TrailingData,
    /// A manifest has no resource data to read.
    #[error("a manifest holds no resource data")]
    NoData,
    /// The package has no resource with this name.
    #[error("package has no resource {0:?}")]
    MissingResource(String),
    /// A resource's bytes do not match its hash.
    #[error("resource {0:?} failed hash verification")]
    HashMismatch(String),
    /// A resource supplied for assembly has a different size from its table entry.
    #[error("resource {name:?} is {actual} bytes, but the manifest says {expected}")]
    WrongSize {
        /// The resource name.
        name: String,
        /// The size in the table.
        expected: u64,
        /// The size supplied.
        actual: u64,
    },
}

/// One resource in a package's table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    /// The resource name, for example `dom.bin` or `style/00-global.bin`.
    pub name: String,
    /// Where the resource starts, counted from the start of the data section.
    pub offset: u64,
    /// The resource's size in bytes.
    pub size: u64,
    /// The SHA-256 hash of the resource's bytes.
    pub hash: ResourceHash,
}

/// Whether a package carries a valid publisher signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    /// No publisher key or signature.
    Unsigned,
    /// The signature is valid for the embedded publisher key.
    Valid,
    /// The signature does not match: the package was changed after signing, or is forged.
    Invalid,
}

/// An opened `.vpp` package or `.vppm` manifest.
#[derive(Debug, Clone)]
pub struct Package {
    bytes: Vec<u8>,
    manifest: Manifest,
    entries: Vec<PackageEntry>,
    publisher: Option<(PublicKey, Signature)>,
    /// End of the signed region: the manifest and the resource table.
    signed_end: usize,
    /// Start of the data section; the `.vppm` manifest is everything before it.
    data_start: usize,
    has_data: bool,
}

impl Package {
    /// Builds a package from a manifest and its resources, signed with `key` if given.
    ///
    /// Resources are stored in name order, which a `BTreeMap` provides, so the
    /// same inputs always give the same bytes.
    ///
    /// # Panics
    ///
    /// If a name or manifest field is longer than [`crate::MAX_STRING`] bytes,
    /// or there are more than [`crate::MAX_COUNT`] resources: packages no reader would accept.
    pub fn build(
        manifest: &Manifest,
        resources: &BTreeMap<String, Vec<u8>>,
        key: Option<&SigningKey>,
    ) -> Vec<u8> {
        let count = u32::try_from(resources.len())
            .ok()
            .filter(|&n| n <= crate::MAX_COUNT)
            .expect("more resources than a package can hold");

        let mut w = ByteWriter::new();
        w.magic(&PACKAGE_MAGIC);
        w.u16(PACKAGE_VERSION);
        w.str(&manifest.site_id);
        w.str(&manifest.site_name);
        w.str(&manifest.site_version);
        w.str(&manifest.page);
        w.str(&manifest.window);

        w.u32(count);
        let mut offset = 0u64;
        for (name, bytes) in resources {
            w.str(name);
            w.u64(offset);
            w.u64(bytes.len() as u64);
            w.raw(ResourceHash::of(bytes).as_bytes());
            offset += bytes.len() as u64;
        }

        // Everything written so far is the signed region.
        match key {
            Some(key) => {
                let signature = key.sign(w.as_bytes());
                w.blob(key.public_key().as_bytes());
                w.blob(signature.as_bytes());
            }
            None => {
                w.blob(&[]);
                w.blob(&[]);
            }
        }

        for bytes in resources.values() {
            w.raw(bytes);
        }
        w.into_bytes()
    }

    /// Opens a full `.vpp` package.
    pub fn open(bytes: Vec<u8>) -> Result<Self, PackageError> {
        Self::parse(bytes, true)
    }

    /// Opens a `.vppm` manifest: a package without its data section.
    pub fn open_manifest(bytes: Vec<u8>) -> Result<Self, PackageError> {
        Self::parse(bytes, false)
    }

    /// Builds a full package from a manifest and the resources it lists, as the
    /// updater does after downloading only the resources it was missing.
    ///
    /// Every resource must be present, the right size, and match its hash. The
    /// result is byte for byte the package the publisher built.
    pub fn assemble(
        manifest: &Package,
        resources: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Vec<u8>, PackageError> {
        let header = manifest.manifest_bytes();
        let data_len = manifest
            .entries
            .iter()
            .map(|e| e.offset + e.size)
            .max()
            .unwrap_or(0);
        let data_len = usize::try_from(data_len)
            .map_err(|_| PackageError::OutOfBounds(String::from("(data section)")))?;

        let mut out = Vec::with_capacity(header.len() + data_len);
        out.extend_from_slice(header);
        out.resize(header.len() + data_len, 0);

        for entry in &manifest.entries {
            let bytes = resources
                .get(&entry.name)
                .ok_or_else(|| PackageError::MissingResource(entry.name.clone()))?;
            if bytes.len() as u64 != entry.size {
                return Err(PackageError::WrongSize {
                    name: entry.name.clone(),
                    expected: entry.size,
                    actual: bytes.len() as u64,
                });
            }
            if ResourceHash::of(bytes) != entry.hash {
                return Err(PackageError::HashMismatch(entry.name.clone()));
            }
            // Cannot overflow: offset + size <= data_len, which fits in usize.
            let start = header.len() + entry.offset as usize;
            out[start..start + bytes.len()].copy_from_slice(bytes);
        }
        Ok(out)
    }

    /// The manifest fields.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The resource table, in stored order.
    pub fn entries(&self) -> &[PackageEntry] {
        &self.entries
    }

    /// The table entry for `name`.
    pub fn entry(&self, name: &str) -> Option<&PackageEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The embedded publisher key, if the package is signed.
    pub fn publisher_key(&self) -> Option<&PublicKey> {
        self.publisher.as_ref().map(|(key, _)| key)
    }

    /// Whether the package is signed, and whether the signature is valid for its embedded key.
    pub fn verify_signature(&self) -> SignatureStatus {
        match &self.publisher {
            None => SignatureStatus::Unsigned,
            Some((key, signature)) => {
                if key.verify(&self.bytes[..self.signed_end], signature) {
                    SignatureStatus::Valid
                } else {
                    SignatureStatus::Invalid
                }
            }
        }
    }

    /// Whether this is a full package with resource data, rather than a `.vppm` manifest.
    pub fn has_data(&self) -> bool {
        self.has_data
    }

    /// Reads resource `name`, checking it against its hash first.
    pub fn read(&self, name: &str) -> Result<&[u8], PackageError> {
        if !self.has_data {
            return Err(PackageError::NoData);
        }
        let entry = self
            .entry(name)
            .ok_or_else(|| PackageError::MissingResource(name.to_owned()))?;
        // In bounds: checked when the package was opened.
        let start = self.data_start + entry.offset as usize;
        let bytes = &self.bytes[start..start + entry.size as usize];
        if ResourceHash::of(bytes) != entry.hash {
            return Err(PackageError::HashMismatch(name.to_owned()));
        }
        Ok(bytes)
    }

    /// The `.vppm` manifest bytes: everything before the data section.
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.bytes[..self.data_start]
    }

    fn parse(bytes: Vec<u8>, has_data: bool) -> Result<Self, PackageError> {
        let mut r = ByteReader::new(&bytes);
        r.magic(&PACKAGE_MAGIC)
            .map_err(|_| PackageError::NotAPackage)?;
        let version = r.u16()?;
        if version != PACKAGE_VERSION {
            return Err(PackageError::UnsupportedVersion {
                found: version,
                supported: PACKAGE_VERSION,
            });
        }

        let manifest = Manifest {
            site_id: r.str()?.to_owned(),
            site_name: r.str()?.to_owned(),
            site_version: r.str()?.to_owned(),
            page: r.str()?.to_owned(),
            window: r.str()?.to_owned(),
        };

        let count = r.count()?;
        let mut entries = Vec::with_capacity(count as usize);
        let mut names = HashSet::new();
        for _ in 0..count {
            let name = r.str()?;
            let entry = PackageEntry {
                name: name.to_owned(),
                offset: r.u64()?,
                size: r.u64()?,
                hash: ResourceHash::from_bytes(r.array()?),
            };
            if name.is_empty() {
                return Err(PackageError::EmptyResourceName);
            }
            if !names.insert(name) {
                return Err(PackageError::DuplicateResource(entry.name));
            }
            entries.push(entry);
        }
        let signed_end = r.position();

        let key = r.blob()?;
        let signature = r.blob()?;
        let publisher = match (key.len(), signature.len()) {
            (0, 0) => None,
            (32, 64) => Some((
                PublicKey::from_bytes(key.try_into().expect("length checked")),
                Signature::from_bytes(signature.try_into().expect("length checked")),
            )),
            _ => return Err(PackageError::MalformedSignature),
        };
        let data_start = r.position();

        if has_data {
            let data_len = (bytes.len() - data_start) as u64;
            for entry in &entries {
                let fits = entry.offset <= data_len && entry.size <= data_len - entry.offset;
                if !fits {
                    return Err(PackageError::OutOfBounds(entry.name.clone()));
                }
            }
        } else if !r.is_at_end() {
            return Err(PackageError::TrailingData);
        }

        Ok(Self {
            bytes,
            manifest,
            entries,
            publisher,
            signed_end,
            data_start,
            has_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            site_id: "org.vpp.test".into(),
            site_name: "Test".into(),
            site_version: "1.0.0".into(),
            page: "home".into(),
            window: r##"{"theme": "#2563eb"}"##.into(),
        }
    }

    fn resources() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            ("style/00-global.bin".into(), b"body{}".to_vec()),
            ("dom.bin".into(), b"<p>hi</p>".to_vec()),
            ("code/00-app.bin".into(), vec![]),
        ])
    }

    fn key() -> SigningKey {
        SigningKey::from_seed([42; 32])
    }

    fn signed() -> Vec<u8> {
        Package::build(&manifest(), &resources(), Some(&key()))
    }

    /// The package with one byte changed. Panics if `at` is out of range.
    fn flipped(mut bytes: Vec<u8>, at: usize) -> Vec<u8> {
        bytes[at] ^= 1;
        bytes
    }

    /// The bytes of a hand-made package whose resource table is given by `table`.
    fn with_table(table: impl FnOnce(&mut ByteWriter)) -> Vec<u8> {
        let mut w = ByteWriter::new();
        w.magic(&PACKAGE_MAGIC);
        w.u16(PACKAGE_VERSION);
        for field in ["id", "name", "1.0.0", "home", ""] {
            w.str(field);
        }
        table(&mut w);
        w.blob(&[]);
        w.blob(&[]);
        w.into_bytes()
    }

    #[test]
    fn layout_matches_the_specification() {
        // Written out from docs/formats/package.md, independently of ByteWriter.
        let manifest = Manifest {
            site_id: "s".into(),
            site_name: "n".into(),
            site_version: "v".into(),
            page: "p".into(),
            window: String::new(),
        };
        let resources = BTreeMap::from([("r".to_string(), b"xy".to_vec())]);
        let mut expected = Vec::new();
        expected.extend_from_slice(b"VPPK");
        expected.extend_from_slice(&[3, 0]);
        for field in [&b"s"[..], b"n", b"v", b"p", b""] {
            expected.extend_from_slice(&(field.len() as u32).to_le_bytes());
            expected.extend_from_slice(field);
        }
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"r");
        expected.extend_from_slice(&0u64.to_le_bytes());
        expected.extend_from_slice(&2u64.to_le_bytes());
        expected.extend_from_slice(ResourceHash::of(b"xy").as_bytes());
        expected.extend_from_slice(&0u32.to_le_bytes()); // no key
        expected.extend_from_slice(&0u32.to_le_bytes()); // no signature
        expected.extend_from_slice(b"xy");

        assert_eq!(Package::build(&manifest, &resources, None), expected);
    }

    #[test]
    fn round_trips() {
        let package = Package::open(signed()).unwrap();
        assert_eq!(package.manifest(), &manifest());
        assert!(package.has_data());
        let names: Vec<_> = package.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["code/00-app.bin", "dom.bin", "style/00-global.bin"]);
        for (name, bytes) in resources() {
            assert_eq!(package.read(&name).unwrap(), bytes.as_slice());
        }
    }

    #[test]
    fn building_is_deterministic() {
        assert_eq!(signed(), signed());
    }

    #[test]
    fn signed_package_verifies() {
        let package = Package::open(signed()).unwrap();
        assert_eq!(package.verify_signature(), SignatureStatus::Valid);
        assert_eq!(package.publisher_key(), Some(&key().public_key()));
    }

    #[test]
    fn unsigned_package_says_so() {
        let package = Package::open(Package::build(&manifest(), &resources(), None)).unwrap();
        assert_eq!(package.verify_signature(), SignatureStatus::Unsigned);
        assert_eq!(package.publisher_key(), None);
    }

    #[test]
    fn changed_manifest_breaks_the_signature() {
        // Byte 10 is inside the site id, in the signed region.
        let package = Package::open(flipped(signed(), 10)).unwrap();
        assert_eq!(package.verify_signature(), SignatureStatus::Invalid);
    }

    #[test]
    fn changed_signature_breaks_the_signature() {
        let bytes = signed();
        let original = Package::open(bytes.clone()).unwrap();
        let in_signature = original.manifest_bytes().len() - 1;
        let package = Package::open(flipped(bytes, in_signature)).unwrap();
        assert_eq!(package.verify_signature(), SignatureStatus::Invalid);
    }

    #[test]
    fn changed_data_fails_its_hash_but_not_the_others() {
        let bytes = signed();
        let last = bytes.len() - 1; // inside style/00-global.bin, the last resource
        let package = Package::open(flipped(bytes, last)).unwrap();
        assert_eq!(package.verify_signature(), SignatureStatus::Valid);
        assert_eq!(
            package.read("style/00-global.bin"),
            Err(PackageError::HashMismatch("style/00-global.bin".into()))
        );
        assert!(package.read("dom.bin").is_ok());
    }

    #[test]
    fn manifest_is_the_package_without_data() {
        let package = Package::open(signed()).unwrap();
        let manifest = Package::open_manifest(package.manifest_bytes().to_vec()).unwrap();
        assert!(!manifest.has_data());
        assert_eq!(manifest.manifest(), package.manifest());
        assert_eq!(manifest.entries(), package.entries());
        assert_eq!(manifest.verify_signature(), SignatureStatus::Valid);
        assert_eq!(manifest.read("dom.bin"), Err(PackageError::NoData));
    }

    #[test]
    fn manifest_with_trailing_data_is_refused() {
        let package = Package::open(signed()).unwrap();
        let mut bytes = package.manifest_bytes().to_vec();
        bytes.push(0);
        assert_eq!(
            Package::open_manifest(bytes).unwrap_err(),
            PackageError::TrailingData
        );
    }

    #[test]
    fn assembling_from_manifest_and_resources_gives_the_original_bytes() {
        let bytes = signed();
        let manifest = Package::open_manifest(
            Package::open(bytes.clone())
                .unwrap()
                .manifest_bytes()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(Package::assemble(&manifest, &resources()).unwrap(), bytes);
    }

    #[test]
    fn assembling_refuses_missing_wrong_size_or_wrong_content() {
        let manifest = Package::open(signed()).unwrap();

        let mut missing = resources();
        missing.remove("dom.bin");
        assert_eq!(
            Package::assemble(&manifest, &missing),
            Err(PackageError::MissingResource("dom.bin".into()))
        );

        let mut wrong_size = resources();
        wrong_size.insert("dom.bin".into(), b"x".to_vec());
        assert!(matches!(
            Package::assemble(&manifest, &wrong_size),
            Err(PackageError::WrongSize { .. })
        ));

        let mut wrong_content = resources();
        wrong_content.insert("dom.bin".into(), b"<p>HI</p>".to_vec());
        assert_eq!(
            Package::assemble(&manifest, &wrong_content),
            Err(PackageError::HashMismatch("dom.bin".into()))
        );
    }

    #[test]
    fn other_files_are_not_packages() {
        assert_eq!(
            Package::open(b"PK\x03\x04 a zip file".to_vec()).unwrap_err(),
            PackageError::NotAPackage
        );
        assert_eq!(
            Package::open(Vec::new()).unwrap_err(),
            PackageError::NotAPackage
        );
    }

    #[test]
    fn other_versions_are_refused() {
        let mut bytes = signed();
        bytes[4] = 4;
        assert_eq!(
            Package::open(bytes).unwrap_err(),
            PackageError::UnsupportedVersion {
                found: 4,
                supported: 3
            }
        );
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let bytes = signed();
        let data_start = Package::open(bytes.clone()).unwrap().manifest_bytes().len();
        for len in 0..bytes.len() {
            let cut = bytes[..len].to_vec();
            if len < data_start {
                assert!(Package::open(cut.clone()).is_err(), "length {len}");
                assert!(Package::open_manifest(cut).is_err(), "length {len}");
            } else {
                // Only the data is cut: some resource now lies outside the file.
                assert!(
                    matches!(Package::open(cut), Err(PackageError::OutOfBounds(_))),
                    "length {len}"
                );
            }
        }
    }

    #[test]
    fn empty_and_duplicate_names_are_refused() {
        let empty = with_table(|w| {
            w.u32(1);
            w.str("");
            w.u64(0);
            w.u64(0);
            w.raw(&[0; 32]);
        });
        assert_eq!(
            Package::open(empty).unwrap_err(),
            PackageError::EmptyResourceName
        );

        let duplicate = with_table(|w| {
            w.u32(2);
            for _ in 0..2 {
                w.str("dom.bin");
                w.u64(0);
                w.u64(0);
                w.raw(ResourceHash::of(b"").as_bytes());
            }
        });
        assert_eq!(
            Package::open(duplicate).unwrap_err(),
            PackageError::DuplicateResource("dom.bin".into())
        );
    }

    #[test]
    fn resource_outside_the_file_is_refused() {
        // INTENTIONAL: hostile input. The entry claims u64::MAX bytes at offset 1; the offset + size overflow must be caught.
        let hostile = with_table(|w| {
            w.u32(1);
            w.str("dom.bin");
            w.u64(1);
            w.u64(u64::MAX);
            w.raw(&[0; 32]);
        });
        assert_eq!(
            Package::open(hostile).unwrap_err(),
            PackageError::OutOfBounds("dom.bin".into())
        );
    }

    #[test]
    fn half_a_signature_is_refused() {
        let mut w = ByteWriter::new();
        w.magic(&PACKAGE_MAGIC);
        w.u16(PACKAGE_VERSION);
        for field in ["id", "name", "1.0.0", "home", ""] {
            w.str(field);
        }
        w.u32(0);
        w.blob(key().public_key().as_bytes());
        w.blob(&[]);
        assert_eq!(
            Package::open(w.into_bytes()).unwrap_err(),
            PackageError::MalformedSignature
        );
    }

    #[test]
    fn missing_resource_is_reported_by_name() {
        let package = Package::open(signed()).unwrap();
        assert_eq!(
            package.read("nope.bin"),
            Err(PackageError::MissingResource("nope.bin".into()))
        );
    }
}

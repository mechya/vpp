//! SHA-256 resource hashes.
//!
//! A resource is identified by its hash, never by name alone
//! (`ARCHITECTURE.md`, invariant 4). The hex form names resource files on a
//! server and in the viewer's store.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::hex;

/// The SHA-256 hash of a resource.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceHash([u8; 32]);

impl ResourceHash {
    /// Hashes `data`.
    pub fn of(data: &[u8]) -> Self {
        Self(Sha256::digest(data).into())
    }

    /// Wraps a hash read from a file.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The 32 hash bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Lowercase hex, as used for resource file names.
impl fmt::Display for ResourceHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(&self.0))
    }
}

impl fmt::Debug for ResourceHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceHash({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test vectors from FIPS 180-2.
    #[test]
    fn matches_the_standard() {
        assert_eq!(
            ResourceHash::of(b"").to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            ResourceHash::of(b"abc").to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn different_data_different_hash() {
        assert_ne!(ResourceHash::of(b"a"), ResourceHash::of(b"b"));
    }
}

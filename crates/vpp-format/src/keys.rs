//! Publisher keys: Ed25519 key pairs and their text files.
//!
//! The secret key file holds the 32-byte seed and stays on the publisher's
//! machine; the public key travels inside every package. Both files are a
//! comment line starting with `#`, then the key in hex, the same as the removed
//! C++ tools wrote them. Reading and writing the files themselves is left to the
//! tools; this module only converts to and from their text.

use std::fmt;

use thiserror::Error;

use crate::hex;

const SECRET_FILE_COMMENT: &str =
    "# VPP publisher secret key (Ed25519 seed). Keep this file private.";
const PUBLIC_FILE_COMMENT: &str = "# VPP publisher public key (Ed25519).";

/// Why a key could not be made or read.
#[derive(Debug, Error)]
pub enum KeyError {
    /// The operating system could not supply random bytes for a new key.
    #[error("no random source available to generate a key: {0}")]
    NoRandomSource(getrandom::Error),
    /// The key file has no line other than comments and blank lines.
    #[error("key file contains no key")]
    NoKey,
    /// The key line is not 64 hexadecimal digits.
    #[error("key file must contain the key as 64 hexadecimal digits")]
    NotHex,
}

/// A publisher's secret signing key.
///
/// Its `Debug` output shows only the public key, so the secret cannot leak into logs.
#[derive(Clone)]
pub struct SigningKey(pub(crate) ed25519_dalek::SigningKey);

impl SigningKey {
    /// A new key from the operating system's random source.
    pub fn generate() -> Result<Self, KeyError> {
        let mut seed = [0; 32];
        getrandom::fill(&mut seed).map_err(KeyError::NoRandomSource)?;
        Ok(Self::from_seed(seed))
    }

    /// The key for a 32-byte seed, as stored in the secret key file.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(ed25519_dalek::SigningKey::from_bytes(&seed))
    }

    /// The 32-byte seed.
    pub fn seed(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// The matching public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().to_bytes())
    }

    /// The contents of a secret key file.
    pub fn to_file_text(&self) -> String {
        format!("{SECRET_FILE_COMMENT}\n{}\n", hex::encode(&self.seed()))
    }

    /// Reads the contents of a secret key file.
    pub fn from_file_text(text: &str) -> Result<Self, KeyError> {
        Ok(Self::from_seed(key_from_file_text(text)?))
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SigningKey(public {})", self.public_key())
    }
}

/// A publisher's public key, as carried in packages and pinned per site.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    /// Wraps 32 key bytes read from a package or a trust record.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The 32 key bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The contents of a public key file.
    pub fn to_file_text(&self) -> String {
        format!("{PUBLIC_FILE_COMMENT}\n{self}\n")
    }

    /// Reads the contents of a public key file.
    pub fn from_file_text(text: &str) -> Result<Self, KeyError> {
        Ok(Self(key_from_file_text(text)?))
    }
}

/// Lowercase hex.
impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(&self.0))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({self})")
    }
}

/// The first line that is neither blank nor a `#` comment, decoded from hex.
fn key_from_file_text(text: &str) -> Result<[u8; 32], KeyError> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .ok_or(KeyError::NoKey)?;
    hex::decode(line).ok_or(KeyError::NotHex)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8032, section 7.1, test 1.
    const RFC_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const RFC_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    fn rfc_key() -> SigningKey {
        SigningKey::from_seed(hex::decode(RFC_SEED).unwrap())
    }

    #[test]
    fn public_key_matches_the_standard() {
        assert_eq!(rfc_key().public_key().to_string(), RFC_PUBLIC);
    }

    #[test]
    fn generated_keys_differ() {
        let a = SigningKey::generate().unwrap();
        let b = SigningKey::generate().unwrap();
        assert_ne!(a.seed(), b.seed());
    }

    #[test]
    fn key_files_round_trip() {
        let key = rfc_key();
        let secret = key.to_file_text();
        assert!(secret.starts_with("# "));
        assert_eq!(
            SigningKey::from_file_text(&secret).unwrap().seed(),
            key.seed()
        );

        let public = key.public_key().to_file_text();
        assert_eq!(
            PublicKey::from_file_text(&public).unwrap(),
            key.public_key()
        );
    }

    #[test]
    fn key_file_skips_comments_blank_lines_and_spaces() {
        let text = format!("# one\n\n  # two\n  {RFC_SEED}  \r\n");
        assert_eq!(
            SigningKey::from_file_text(&text)
                .unwrap()
                .public_key()
                .to_string(),
            RFC_PUBLIC
        );
    }

    #[test]
    fn bad_key_files_are_refused() {
        assert!(matches!(
            SigningKey::from_file_text("# only a comment\n"),
            Err(KeyError::NoKey)
        ));
        assert!(matches!(
            SigningKey::from_file_text("abcd\n"),
            Err(KeyError::NotHex)
        ));
        assert!(matches!(
            PublicKey::from_file_text(&RFC_PUBLIC.replace('d', "x")),
            Err(KeyError::NotHex)
        ));
    }

    #[test]
    fn debug_never_shows_the_secret() {
        let shown = format!("{:?}", rfc_key());
        assert!(!shown.contains(RFC_SEED));
        assert!(shown.contains(RFC_PUBLIC));
    }
}

//! Ed25519 publisher signatures: signing with a [`SigningKey`] and verifying
//! with a [`PublicKey`].

use std::fmt;

use ed25519_dalek::Signer;

use crate::hex;
use crate::keys::{PublicKey, SigningKey};

/// A 64-byte Ed25519 signature.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature([u8; 64]);

impl Signature {
    /// Wraps 64 signature bytes read from a package.
    pub const fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// The 64 signature bytes.
    pub const fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", hex::encode(&self.0))
    }
}

impl SigningKey {
    /// Signs `message`. Ed25519 is deterministic: the same key and message give the same signature.
    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.0.sign(message).to_bytes())
    }
}

impl PublicKey {
    /// Whether `signature` is a valid signature of `message` by this key.
    ///
    /// Uses strict verification, which also refuses weak keys and malleable
    /// signatures. A key that is not a valid curve point verifies nothing.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(self.as_bytes()) else {
            return false;
        };
        let signature = ed25519_dalek::Signature::from_bytes(&signature.0);
        key.verify_strict(message, &signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8032, section 7.1, test 1: the empty message.
    const RFC_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const RFC_SIGNATURE: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

    fn rfc_key() -> SigningKey {
        SigningKey::from_seed(hex::decode(RFC_SEED).unwrap())
    }

    #[test]
    fn signature_matches_the_standard() {
        let signature = rfc_key().sign(b"");
        assert_eq!(hex::encode(signature.as_bytes()), RFC_SIGNATURE);
        assert!(rfc_key().public_key().verify(b"", &signature));
    }

    #[test]
    fn tampered_message_fails() {
        let key = rfc_key();
        let signature = key.sign(b"page");
        assert!(key.public_key().verify(b"page", &signature));
        assert!(!key.public_key().verify(b"pagf", &signature));
    }

    #[test]
    fn tampered_signature_fails() {
        let key = rfc_key();
        let mut bytes = *key.sign(b"page").as_bytes();
        bytes[10] ^= 1;
        assert!(
            !key.public_key()
                .verify(b"page", &Signature::from_bytes(bytes))
        );
    }

    #[test]
    fn another_key_fails() {
        let other = SigningKey::from_seed([7; 32]);
        let signature = rfc_key().sign(b"page");
        assert!(!other.public_key().verify(b"page", &signature));
    }

    #[test]
    fn garbage_key_verifies_nothing() {
        // INTENTIONAL: hostile input. Arbitrary key bytes; verification must fail, not panic.
        let bogus = PublicKey::from_bytes([0xff; 32]);
        assert!(!bogus.verify(b"page", &rfc_key().sign(b"page")));
    }
}

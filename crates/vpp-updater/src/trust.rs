//! Publisher trust: the first key seen for a site id is remembered, and a
//! page of that site signed by any other key is refused (`ARCHITECTURE.md`,
//! invariant 2). Moved here from the C++ viewer, so it is tested on its own.
//!
//! Each site's key is a public key file (`vpp-format`) named after the site:
//! `<folder>/<safe site id>.pub`.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;
use vpp_format::PublicKey;

use crate::store::{safe_name, write_atomic};

/// The pinned publisher keys, one file per site.
#[derive(Debug, Clone)]
pub struct Trust {
    dir: PathBuf,
}

/// What [`Trust::check`] decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDecision {
    /// The key is the one pinned for this site.
    Known,
    /// No key was pinned for this site; this one now is.
    FirstUse,
}

/// Why a publisher was refused.
#[derive(Debug, Error)]
pub enum TrustError {
    /// The site already has a different key.
    #[error(
        "publisher key changed for {site_id}: expected {expected}, got {offered}. \
         If the site really changed its key, delete {} and open it again.",
        file.display()
    )]
    KeyChanged {
        /// The site.
        site_id: String,
        /// The pinned key.
        expected: PublicKey,
        /// The key the page is signed with.
        offered: PublicKey,
        /// The file holding the pinned key.
        file: PathBuf,
    },
    /// The trust folder could not be read or written.
    #[error("cannot use the trust record {}: {source}", file.display())]
    Io {
        /// The file involved.
        file: PathBuf,
        /// What went wrong.
        source: std::io::Error,
    },
}

impl Trust {
    /// Pinned keys kept in `dir`, which is created when first written to.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The folder the pinned keys are in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Accepts `key` for `site_id` if it is the pinned key, or pins it if the
    /// site has none yet. Refuses any other key.
    pub fn check(&self, site_id: &str, key: &PublicKey) -> Result<TrustDecision, TrustError> {
        let file = self.dir.join(format!("{}.pub", safe_name(site_id)));
        match fs::read_to_string(&file) {
            Ok(text) => match PublicKey::from_file_text(&text) {
                Ok(pinned) if pinned == *key => Ok(TrustDecision::Known),
                Ok(pinned) => Err(TrustError::KeyChanged {
                    site_id: site_id.to_owned(),
                    expected: pinned,
                    offered: *key,
                    file,
                }),
                // A damaged record never silently becomes a new pin.
                Err(e) => Err(TrustError::Io {
                    file,
                    source: std::io::Error::other(e.to_string()),
                }),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                write_atomic(&file, key.to_file_text().as_bytes())
                    .map_err(|source| TrustError::Io { file, source })?;
                Ok(TrustDecision::FirstUse)
            }
            Err(source) => Err(TrustError::Io { file, source }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;
    use vpp_format::SigningKey;

    fn key(seed: u8) -> PublicKey {
        SigningKey::from_seed([seed; 32]).public_key()
    }

    #[test]
    fn first_key_is_pinned_then_known() {
        let dir = TestDir::new("trust");
        let trust = Trust::new(dir.path().join("trust"));
        assert_eq!(
            trust.check("org.a", &key(1)).unwrap(),
            TrustDecision::FirstUse
        );
        assert_eq!(trust.check("org.a", &key(1)).unwrap(), TrustDecision::Known);
    }

    #[test]
    fn a_different_key_is_refused_and_the_pin_kept() {
        let dir = TestDir::new("trust-changed");
        let trust = Trust::new(dir.path());
        trust.check("org.a", &key(1)).unwrap();
        let err = trust.check("org.a", &key(2)).unwrap_err();
        assert!(matches!(err, TrustError::KeyChanged { .. }));
        assert!(err.to_string().contains("delete"));
        assert_eq!(trust.check("org.a", &key(1)).unwrap(), TrustDecision::Known);
    }

    #[test]
    fn sites_are_pinned_separately() {
        let dir = TestDir::new("trust-sites");
        let trust = Trust::new(dir.path());
        trust.check("org.a", &key(1)).unwrap();
        assert_eq!(
            trust.check("org.b", &key(2)).unwrap(),
            TrustDecision::FirstUse
        );
    }

    #[test]
    fn a_damaged_record_is_an_error_not_a_new_pin() {
        let dir = TestDir::new("trust-damaged");
        let trust = Trust::new(dir.path());
        fs::write(dir.path().join("org.a.pub"), "not a key").unwrap();
        assert!(matches!(
            trust.check("org.a", &key(1)),
            Err(TrustError::Io { .. })
        ));
    }
}

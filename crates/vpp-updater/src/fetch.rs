//! Fetching a URL: the one boundary between the updater and the network, so
//! tests can stand in for a server (`CONTRIBUTING.md` allows a trait object
//! here, "the HTTP client for tests").

use thiserror::Error;

/// Fetches the bytes at a URL.
pub trait Fetch {
    /// GETs `url`, which uses `http://` or `https://`. Anything other than a
    /// complete `200 OK` response is an error.
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}

/// Why a URL could not be fetched.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FetchError {
    /// The server answered with a status other than 200.
    #[error("{url}: HTTP status {status}")]
    Status {
        /// The URL requested.
        url: String,
        /// The status the server sent.
        status: u16,
    },
    /// No answer: the server is unreachable, the connection failed, or it timed out.
    #[error("{url}: {reason}")]
    Unreachable {
        /// The URL requested.
        url: String,
        /// What went wrong.
        reason: String,
    },
}

/// Whether `text` is a page address the updater handles: `http://`,
/// `https://`, or `vpp://`.
pub fn is_remote_url(text: &str) -> bool {
    ["http://", "https://", "vpp://"]
        .iter()
        .any(|scheme| text.starts_with(scheme))
}

/// The URL to fetch for a page address: `vpp://` is an alias for `https://`.
pub fn fetch_url(address: &str) -> String {
    match address.strip_prefix("vpp://") {
        Some(rest) => format!("https://{rest}"),
        None => address.to_owned(),
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! A server held in memory, for tests.

    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::{Fetch, FetchError};

    /// Serves fixed bytes per URL and records every request.
    #[derive(Default)]
    pub(crate) struct FakeServer {
        pub(crate) files: BTreeMap<String, Vec<u8>>,
        pub(crate) requests: RefCell<Vec<String>>,
        /// When set, every request fails as if the network were down.
        pub(crate) offline: bool,
    }

    impl Fetch for FakeServer {
        fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            self.requests.borrow_mut().push(url.to_owned());
            if self.offline {
                return Err(FetchError::Unreachable {
                    url: url.to_owned(),
                    reason: "offline".into(),
                });
            }
            self.files.get(url).cloned().ok_or(FetchError::Status {
                url: url.to_owned(),
                status: 404,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_page_addresses() {
        assert!(is_remote_url("https://example.com/site/home.vpp"));
        assert!(is_remote_url("vpp://example.com/site/home.vpp"));
        assert!(!is_remote_url("C:\\sites\\home.vpp"));
        assert!(!is_remote_url("ftp://example.com/home.vpp"));
    }

    #[test]
    fn vpp_scheme_means_https() {
        assert_eq!(
            fetch_url("vpp://example.com/a.vpp"),
            "https://example.com/a.vpp"
        );
        assert_eq!(
            fetch_url("http://localhost/a.vpp"),
            "http://localhost/a.vpp"
        );
    }
}

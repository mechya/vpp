//! The real network: HTTP and HTTPS through `ureq`, with `rustls` and the
//! operating system's certificate store.
//!
//! Timeouts are short, as in the C++ client, so an unreachable server fails
//! fast and the installed copy of a page runs instead.

use std::sync::Arc;
use std::time::Duration;

use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::fetch::{Fetch, FetchError};

/// The largest response accepted: large enough for any page, small enough
/// that a hostile server cannot exhaust memory.
pub const MAX_RESPONSE: u64 = 256 * 1024 * 1024;

/// Fetches over HTTP and HTTPS.
pub struct HttpFetcher {
    agent: Agent,
}

impl HttpFetcher {
    /// A client that trusts the certificates the operating system trusts,
    /// and uses the system's proxy environment settings.
    pub fn new() -> Self {
        let crypto = Arc::new(rustls::crypto::ring::default_provider());
        let tls = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .unversioned_rustls_crypto_provider(crypto)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let agent = Agent::config_builder()
            .tls_config(tls)
            .user_agent(concat!("VPP/", env!("CARGO_PKG_VERSION")))
            // Resolve and connect quickly, or treat the server as unreachable.
            .timeout_resolve(Some(Duration::from_secs(5)))
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_recv_response(Some(Duration::from_secs(10)))
            .timeout_recv_body(Some(Duration::from_secs(30)))
            .build()
            .new_agent();
        Self { agent }
    }
}

impl Default for HttpFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetch for HttpFetcher {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let unreachable = |e: ureq::Error| FetchError::Unreachable {
            url: url.to_owned(),
            reason: e.to_string(),
        };
        let response = match self.agent.get(url).call() {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(status)) => {
                return Err(FetchError::Status {
                    url: url.to_owned(),
                    status,
                });
            }
            Err(e) => return Err(unreachable(e)),
        };
        let status = response.status().as_u16();
        if status != 200 {
            return Err(FetchError::Status {
                url: url.to_owned(),
                status,
            });
        }
        response
            .into_body()
            .with_config()
            .limit(MAX_RESPONSE)
            .read_to_vec()
            .map_err(unreachable)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    /// A one-request-per-connection HTTP server on localhost, answering `/ok`
    /// with a body and anything else with 404.
    fn serve(body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut request_line = String::new();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                reader.read_line(&mut request_line).unwrap();
                // Skip the headers.
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 2 {
                    line.clear();
                }
                let (status, content) = if request_line.starts_with("GET /ok ") {
                    ("200 OK", body)
                } else {
                    ("404 Not Found", &b""[..])
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(content).unwrap();
            }
        });
        format!("http://{address}")
    }

    #[test]
    fn fetches_a_file() {
        let base = serve(b"page bytes");
        let body = HttpFetcher::new().get(&format!("{base}/ok")).unwrap();
        assert_eq!(body, b"page bytes");
    }

    #[test]
    fn other_statuses_are_errors() {
        let base = serve(b"");
        let url = format!("{base}/missing");
        assert_eq!(
            HttpFetcher::new().get(&url).unwrap_err(),
            FetchError::Status { url, status: 404 }
        );
    }

    #[test]
    fn an_unreachable_server_fails_quickly() {
        // Nothing listens on this port once the listener is dropped.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = HttpFetcher::new()
            .get(&format!("http://127.0.0.1:{port}/ok"))
            .unwrap_err();
        assert!(matches!(err, FetchError::Unreachable { .. }), "{err}");
    }
}

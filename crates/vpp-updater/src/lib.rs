//! Page store and updates.
//!
//! Keeps visited pages on disk, pins each site's publisher key on first use,
//! checks manifests, and downloads only the resources whose hashes changed.
//! It refuses older versions.
//!
//! # Where it sits
//!
//! Between the network and the viewer. Every page the viewer opens from a URL
//! comes through here. The viewer chooses the store and trust folders (from
//! `vpp-platform`) and passes them in; this crate never asks the operating
//! system where to put things.
//!
//! # Not in this crate
//!
//! The byte formats themselves (`vpp-format`). Signature checks are
//! `vpp-format`'s; deciding whether a publisher is trusted is this crate's.
//!
//! # Where to start reading
//!
//! [`Store::sync`] in `sync.rs`, with `docs/reference/updater.md` beside it.
//! `store.rs` is the layout on disk, `trust.rs` the pinned publisher keys,
//! `version.rs` rollback ordering, and `fetch.rs` and `http.rs` the network.

mod fetch;
mod http;
mod store;
mod sync;
#[cfg(test)]
mod test_dir;
mod trust;
mod version;

pub use fetch::{Fetch, FetchError, fetch_url, is_remote_url};
pub use http::{HttpFetcher, MAX_RESPONSE};
pub use store::{Store, safe_name};
pub use sync::{SyncError, SyncResult};
pub use trust::{Trust, TrustDecision, TrustError};
pub use version::compare_versions;

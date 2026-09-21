//! The updater against a real HTTP server on localhost, serving a site in the
//! layout `vpppack --publish` writes (`docs/rust-port.md` §8, step 4): two
//! versions of a page, only changed resources downloaded, rollback refused,
//! and the installed copy used once the server is gone.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use vpp_format::{Manifest, Package, SigningKey};
use vpp_updater::{HttpFetcher, Store, SyncError};

/// A static file server for `root`. While `down` is set it drops every connection.
fn serve(root: PathBuf, down: Arc<AtomicBool>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            if down.load(Ordering::SeqCst) {
                continue; // dropping the stream closes the connection
            }
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            let mut header = String::new();
            while reader
                .read_line(&mut header)
                .map(|n| n > 2)
                .unwrap_or(false)
            {
                header.clear();
            }
            let path = request_line
                .split(' ')
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/');
            let (status, body) = match fs::read(root.join(path)) {
                Ok(body) if !path.contains("..") => ("200 OK", body),
                _ => ("404 Not Found", Vec::new()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{address}")
}

/// Publishes the home page into `site`, as `vpppack --publish` lays it out.
fn publish(site: &Path, version: &str, style: &[u8]) {
    let manifest = Manifest {
        site_id: "org.example.served".into(),
        site_name: "Served".into(),
        site_version: version.into(),
        page: "home".into(),
        window: String::new(),
    };
    let resources = BTreeMap::from([
        ("dom.bin".to_string(), b"the same dom".to_vec()),
        ("style/00-global.bin".to_string(), style.to_vec()),
    ]);
    let bytes = Package::build(&manifest, &resources, Some(&SigningKey::from_seed([9; 32])));
    let package = Package::open(bytes.clone()).unwrap();
    fs::create_dir_all(site.join("res")).unwrap();
    fs::write(site.join("home.vpp"), &bytes).unwrap();
    fs::write(site.join("home.vppm"), package.manifest_bytes()).unwrap();
    for entry in package.entries() {
        fs::write(
            site.join("res").join(entry.hash.to_string()),
            package.read(&entry.name).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn install_update_rollback_and_offline_over_http() {
    let temp = std::env::temp_dir().join(format!("vpp-updater-served-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let site = temp.join("site");
    let store = Store::new(temp.join("store"));
    let down = Arc::new(AtomicBool::new(false));
    let base = serve(temp.clone(), down.clone());
    let page = format!("{base}/site/home.vpp");
    let http = HttpFetcher::new();
    let mut log = |_: &str| {};

    // Version 1 installs both resources.
    publish(&site, "1.0.0", b"style one");
    let first = store.sync(&page, &http, &mut log).unwrap();
    assert!(first.updated);
    assert_eq!(first.downloaded, 2);

    // Version 2 changes only the stylesheet: one download.
    publish(&site, "1.1.0", b"style two");
    let second = store.sync(&page, &http, &mut log).unwrap();
    assert!(second.updated);
    assert_eq!(second.downloaded, 1);
    assert_eq!(
        fs::read(&second.package_path).unwrap(),
        fs::read(site.join("home.vpp")).unwrap()
    );

    // The server goes back to version 1: refused.
    publish(&site, "1.0.0", b"style one");
    assert!(matches!(
        store.sync(&page, &http, &mut log),
        Err(SyncError::Rollback { .. })
    ));

    // The server disappears: the installed 1.1.0 runs.
    down.store(true, Ordering::SeqCst);
    let offline = store.sync(&page, &http, &mut log).unwrap();
    assert!(offline.offline);
    assert_eq!(offline.version, "1.1.0");

    let _ = fs::remove_dir_all(&temp);
}

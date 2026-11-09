use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use sha2::{Digest, Sha256};

use super::registry::{fetch_registry_package, FetchedPackage, RegistryFetchError};
use super::vsix_tests::TempDirGuard;
use crate::domains::extensions::manifest::ExtensionPackageIdentity;

pub(crate) const PAYLOAD: &[u8] = b"not really a vsix, but bytes are bytes";
const MAX: u64 = 1024 * 1024;

pub(crate) struct Route {
    path: &'static str,
    status: u16,
    body: Vec<u8>,
}

fn identity() -> ExtensionPackageIdentity {
    ExtensionPackageIdentity {
        namespace: "example".into(),
        name: "demo".into(),
        version: "1.0.0".into(),
    }
}

/// Serves `routes` over plain HTTP on an ephemeral port. Unknown paths get 404.
/// The accept loop lives on a detached thread for the test process lifetime.
pub(crate) fn serve(routes: Vec<Route>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub registry");
    let base = format!("http://{}", listener.local_addr().expect("local addr"));
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&chunk[..read]),
                }
            }
            let line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            let path = line
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            let (status, body) = routes
                .iter()
                .find(|route| route.path == path)
                .map(|route| (route.status, route.body.clone()))
                .unwrap_or((404, Vec::new()));
            let head = format!(
                "HTTP/1.1 {status} stub\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    base
}

pub(crate) fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn metadata(version: &str, download: &str, checksum: &str) -> Vec<u8> {
    serde_json::json!({
        "namespace": "example",
        "name": "demo",
        "version": version,
        "files": { "download": download, "sha256": checksum },
    })
    .to_string()
    .into_bytes()
}

pub(crate) fn standard_routes(checksum_body: String, payload: &[u8]) -> Vec<Route> {
    vec![
        Route {
            path: "/api/example/demo/1.0.0",
            status: 200,
            body: metadata("1.0.0", "/files/demo.vsix", "/files/demo.sha256"),
        },
        Route {
            path: "/files/demo.vsix",
            status: 200,
            body: payload.to_vec(),
        },
        Route {
            path: "/files/demo.sha256",
            status: 200,
            body: checksum_body.into_bytes(),
        },
    ]
}

fn expect_error(result: Result<FetchedPackage, RegistryFetchError>) -> RegistryFetchError {
    match result {
        Ok(_) => panic!("expected the registry fetch to fail"),
        Err(error) => error,
    }
}

#[test]
fn fetches_verified_package_into_staging() {
    let base = serve(standard_routes(
        format!("{}  demo.vsix\n", hex_sha256(PAYLOAD)),
        PAYLOAD,
    ));
    let staging = TempDirGuard::new("registry-success");

    let fetched = fetch_registry_package(&base, &identity(), &staging, MAX)
        .expect("registry package should fetch");

    assert_eq!(fetched.sha256, hex_sha256(PAYLOAD));
    assert_eq!(std::fs::read(&fetched.archive).expect("archive"), PAYLOAD);
    assert!(fetched.archive.starts_with(&*staging));
}

#[test]
fn refuses_artifact_urls_outside_the_registry_origin() {
    let base = serve(vec![Route {
        path: "/api/example/demo/1.0.0",
        status: 200,
        body: metadata(
            "1.0.0",
            "https://evil.example/demo.vsix",
            "/files/demo.sha256",
        ),
    }]);
    let staging = TempDirGuard::new("registry-foreign");

    let error = expect_error(fetch_registry_package(&base, &identity(), &staging, MAX));

    assert!(matches!(error, RegistryFetchError::ForeignArtifact));
}

#[test]
fn reports_missing_version_as_not_found() {
    let base = serve(Vec::new());
    let staging = TempDirGuard::new("registry-missing");

    let error = expect_error(fetch_registry_package(&base, &identity(), &staging, MAX));

    assert!(matches!(error, RegistryFetchError::NotFound));
}

#[test]
fn rejects_metadata_that_describes_a_different_version() {
    let base = serve(vec![Route {
        path: "/api/example/demo/1.0.0",
        status: 200,
        body: metadata("2.0.0", "/files/demo.vsix", "/files/demo.sha256"),
    }]);
    let staging = TempDirGuard::new("registry-identity");

    let error = expect_error(fetch_registry_package(&base, &identity(), &staging, MAX));

    assert!(matches!(error, RegistryFetchError::MalformedMetadata));
}

#[test]
fn rejects_malformed_checksum_and_leaves_no_archive() {
    let base = serve(standard_routes("not-a-digest\n".into(), PAYLOAD));
    let staging = TempDirGuard::new("registry-checksum");

    let error = expect_error(fetch_registry_package(&base, &identity(), &staging, MAX));

    assert!(matches!(error, RegistryFetchError::MalformedChecksum));
}

#[test]
fn enforces_the_size_limit_and_removes_partial_downloads() {
    let base = serve(standard_routes(hex_sha256(PAYLOAD), PAYLOAD));
    let staging = TempDirGuard::new("registry-size");

    let error = expect_error(fetch_registry_package(&base, &identity(), &staging, 8));

    assert!(matches!(error, RegistryFetchError::TooLarge));
    assert_eq!(std::fs::read_dir(&*staging).expect("staging").count(), 0);
}

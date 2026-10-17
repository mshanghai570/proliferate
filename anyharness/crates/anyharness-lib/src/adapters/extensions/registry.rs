//! Open VSX package transport for the runtime.
//!
//! Resolves one exact `namespace.name@version` through the registry's metadata
//! document, then fetches the artifact and its SHA-256 sidecar. Every artifact
//! URL must share the registry's origin, redirects are refused, and both
//! downloads are size-bounded. Verification of the archive itself remains the
//! installer's job, using the checksum this module returns.
//!
//! Blocking: call from a blocking worker, never from the async executor.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use super::package::private_file;
use crate::domains::extensions::manifest::ExtensionPackageIdentity;

pub const OPEN_VSX_BASE_URL: &str = "https://open-vsx.org";

const USER_AGENT: &str = "proliferate-anyharness-extensions";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_METADATA_BYTES: u64 = 1024 * 1024;
const MAX_CHECKSUM_BYTES: u64 = 256;

#[derive(Debug, Error)]
pub enum RegistryFetchError {
    #[error("registry request failed")]
    Transport,
    #[error("extension version was not found in the registry")]
    NotFound,
    #[error("registry returned status {0}")]
    Status(u16),
    #[error("registry metadata is malformed or does not match the request")]
    MalformedMetadata,
    #[error("registry artifact URL is outside the registry origin")]
    ForeignArtifact,
    #[error("registry checksum is malformed")]
    MalformedChecksum,
    #[error("registry package exceeds the size limit")]
    TooLarge,
    #[error("local staging failed: {0}")]
    Io(#[from] io::Error),
}

pub struct FetchedPackage {
    /// Private staging file. The caller owns it and must remove it after use.
    pub archive: PathBuf,
    /// Lowercase hex SHA-256 published by the registry for `archive`.
    pub sha256: String,
}

/// Fetches `identity` from the registry at `base`, staging the archive in
/// `staging_dir` under a unique hidden name.
pub fn fetch_registry_package(
    base: &str,
    identity: &ExtensionPackageIdentity,
    staging_dir: &Path,
    max_package_bytes: u64,
) -> Result<FetchedPackage, RegistryFetchError> {
    let base = Url::parse(base).map_err(|_| RegistryFetchError::MalformedMetadata)?;
    let client = Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(Policy::none())
        .user_agent(USER_AGENT)
        .build()
        .map_err(|_| RegistryFetchError::Transport)?;

    let mut metadata_url = base.clone();
    metadata_url
        .path_segments_mut()
        .map_err(|_| RegistryFetchError::MalformedMetadata)?
        .pop_if_empty()
        .extend([
            "api",
            identity.namespace.as_str(),
            identity.name.as_str(),
            identity.version.as_str(),
        ]);
    let metadata_bytes = get_bounded(&client, &metadata_url, MAX_METADATA_BYTES)?;
    let metadata: Value = serde_json::from_slice(&metadata_bytes)
        .map_err(|_| RegistryFetchError::MalformedMetadata)?;
    check_identity(&metadata, identity)?;

    let download_url = resolve_artifact(&base, string_at(&metadata, &["files", "download"])?)?;
    let checksum_url = resolve_artifact(&base, string_at(&metadata, &["files", "sha256"])?)?;

    let checksum_bytes = get_bounded(&client, &checksum_url, MAX_CHECKSUM_BYTES)?;
    let sha256 = parse_checksum(&checksum_bytes)?;

    let archive = staging_dir.join(format!(".registry-download-{}.vsix", Uuid::new_v4()));
    match download_to(&client, &download_url, &archive, max_package_bytes) {
        Ok(()) => Ok(FetchedPackage { archive, sha256 }),
        Err(error) => {
            let _ = fs::remove_file(&archive);
            Err(error)
        }
    }
}

fn check_identity(
    metadata: &Value,
    identity: &ExtensionPackageIdentity,
) -> Result<(), RegistryFetchError> {
    let namespace = string_at(metadata, &["namespace"])?;
    let name = string_at(metadata, &["name"])?;
    let version = string_at(metadata, &["version"])?;
    if namespace.eq_ignore_ascii_case(&identity.namespace)
        && name.eq_ignore_ascii_case(&identity.name)
        && version == identity.version
    {
        Ok(())
    } else {
        Err(RegistryFetchError::MalformedMetadata)
    }
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, RegistryFetchError> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or(RegistryFetchError::MalformedMetadata)?;
    }
    current
        .as_str()
        .ok_or(RegistryFetchError::MalformedMetadata)
}

/// Resolves a registry-advertised artifact URL and refuses any origin other
/// than the registry's own, so metadata cannot redirect the runtime elsewhere.
fn resolve_artifact(base: &Url, raw: &str) -> Result<Url, RegistryFetchError> {
    let url = base
        .join(raw)
        .map_err(|_| RegistryFetchError::MalformedMetadata)?;
    if url.origin() != base.origin() {
        return Err(RegistryFetchError::ForeignArtifact);
    }
    Ok(url)
}

fn parse_checksum(bytes: &[u8]) -> Result<String, RegistryFetchError> {
    let text = std::str::from_utf8(bytes).map_err(|_| RegistryFetchError::MalformedChecksum)?;
    let digest = text
        .split_whitespace()
        .next()
        .ok_or(RegistryFetchError::MalformedChecksum)?
        .to_ascii_lowercase();
    if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(digest)
    } else {
        Err(RegistryFetchError::MalformedChecksum)
    }
}

fn send(client: &Client, url: &Url) -> Result<Response, RegistryFetchError> {
    let response = client
        .get(url.clone())
        .send()
        .map_err(|_| RegistryFetchError::Transport)?;
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        Err(RegistryFetchError::NotFound)
    } else if status.is_success() {
        Ok(response)
    } else {
        Err(RegistryFetchError::Status(status.as_u16()))
    }
}

fn get_bounded(client: &Client, url: &Url, max: u64) -> Result<Vec<u8>, RegistryFetchError> {
    let response = send(client, url)?;
    let mut bytes = Vec::new();
    response
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| RegistryFetchError::Transport)?;
    if bytes.len() as u64 > max {
        return Err(RegistryFetchError::TooLarge);
    }
    Ok(bytes)
}

fn download_to(
    client: &Client,
    url: &Url,
    path: &Path,
    max: u64,
) -> Result<(), RegistryFetchError> {
    let mut response = send(client, url)?;
    if response.content_length().is_some_and(|length| length > max) {
        return Err(RegistryFetchError::TooLarge);
    }
    let mut file = private_file(path)?;
    let copied = io::copy(&mut (&mut response).take(max + 1), &mut file)
        .map_err(|_| RegistryFetchError::Transport)?;
    if copied > max {
        return Err(RegistryFetchError::TooLarge);
    }
    file.sync_all()?;
    Ok(())
}

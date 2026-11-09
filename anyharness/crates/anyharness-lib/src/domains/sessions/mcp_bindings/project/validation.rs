use std::collections::HashSet;

use crate::domains::sessions::mcp_bindings::model::SessionMcpServer;

use super::ProjectMcpError;

fn invalid(detail: &'static str) -> ProjectMcpError {
    ProjectMcpError::Invalid(detail)
}
fn text(value: &str) -> bool {
    value.len() <= 4096 && !value.contains('\0')
}

pub(super) fn validate_and_normalize(
    name: &str,
    server: &mut SessionMcpServer,
) -> Result<(), ProjectMcpError> {
    if name.is_empty()
        || name.len() > 50
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || name.to_ascii_lowercase().starts_with("proliferate_")
    {
        return Err(invalid("Use a name of 1–50 letters, digits, hyphens, or underscores, without the reserved proliferate_ prefix."));
    }
    match server {
        SessionMcpServer::Http(server) => {
            let url =
                url::Url::parse(&server.url).map_err(|_| invalid("Use an HTTP or HTTPS URL."))?;
            if !text(&server.url)
                || !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
            {
                return Err(invalid(
                    "Use an HTTP or HTTPS URL without a fragment or embedded username/password.",
                ));
            }
            if server.headers.len() > 32 {
                return Err(invalid("Use at most 32 headers."));
            }
            let mut names = HashSet::new();
            for header in &server.headers {
                if header.name.len() > 128
                    || !names.insert(header.name.to_ascii_lowercase())
                    || reqwest::header::HeaderName::from_bytes(header.name.as_bytes()).is_err()
                    || !text(&header.value)
                    || reqwest::header::HeaderValue::from_str(&header.value).is_err()
                {
                    return Err(invalid(
                        "Use valid, unique HTTP header names and single-line values.",
                    ));
                }
            }
            server.connection_id = format!("local-project:{name}");
            server.server_name = name.into();
            server.catalog_entry_id = None;
        }
        SessionMcpServer::Stdio(server) => {
            if server.command.trim().is_empty()
                || !text(&server.command)
                || server.command.contains(['\r', '\n'])
                || server.args.len() > 128
                || !server.args.iter().all(|arg| text(arg))
            {
                return Err(invalid("Supply a command and at most 128 arguments, each at most 4096 bytes and without NUL characters."));
            }
            if server.env.len() > 32 {
                return Err(invalid("Use at most 32 environment variables."));
            }
            let mut names = HashSet::new();
            for var in &server.env {
                let mut bytes = var.name.bytes();
                if var.name.len() > 128
                    || !bytes
                        .next()
                        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                    || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    || !names.insert(&var.name)
                    || !text(&var.value)
                {
                    return Err(invalid("Use valid, unique environment variable names and values without NUL characters."));
                }
            }
            server.connection_id = format!("local-project:{name}");
            server.server_name = name.into();
            server.catalog_entry_id = None;
        }
    }
    Ok(())
}

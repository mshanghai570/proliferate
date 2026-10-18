use super::model::SessionMcpServer;
use crate::persistence::data_cipher::{decrypt_bytes, encrypt_bytes, DataCipher};
use anyhow::Context;
use std::fmt;

pub enum SessionMcpBindingsError {
    MissingDataKey,
    Encrypt(anyhow::Error),
    Decrypt(anyhow::Error),
}

impl fmt::Debug for SessionMcpBindingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDataKey => write!(f, "SessionMcpBindingsError::MissingDataKey"),
            Self::Encrypt(error) => f
                .debug_tuple("SessionMcpBindingsError::Encrypt")
                .field(&error.to_string())
                .finish(),
            Self::Decrypt(error) => f
                .debug_tuple("SessionMcpBindingsError::Decrypt")
                .field(&error.to_string())
                .finish(),
        }
    }
}

impl SessionMcpBindingsError {
    pub fn missing_data_key_detail() -> &'static str {
        "ANYHARNESS_DATA_KEY is required when MCP bindings are present."
    }
}

pub fn encrypt_bindings(
    cipher: Option<&DataCipher>,
    bindings: &[SessionMcpServer],
) -> Result<Option<String>, SessionMcpBindingsError> {
    if bindings.is_empty() {
        return Ok(None);
    }

    let Some(cipher) = cipher else {
        return Err(SessionMcpBindingsError::MissingDataKey);
    };

    let plaintext = serde_json::to_vec(bindings)
        .context("serialize MCP bindings")
        .map_err(SessionMcpBindingsError::Encrypt)?;

    encrypt_bytes(cipher, &plaintext)
        .map(Some)
        .map_err(SessionMcpBindingsError::Encrypt)
}

pub fn decrypt_bindings(
    cipher: Option<&DataCipher>,
    ciphertext: Option<&str>,
) -> Result<Vec<SessionMcpServer>, SessionMcpBindingsError> {
    let Some(ciphertext) = ciphertext.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };
    let Some(cipher) = cipher else {
        return Err(SessionMcpBindingsError::MissingDataKey);
    };

    let plaintext = decrypt_bytes(cipher, ciphertext).map_err(SessionMcpBindingsError::Decrypt)?;
    serde_json::from_slice(&plaintext)
        .context("deserialize MCP bindings")
        .map_err(SessionMcpBindingsError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::sessions::mcp_bindings::model::{
        SessionMcpEnvVar, SessionMcpHeader, SessionMcpHttpServer, SessionMcpServer,
        SessionMcpStdioServer,
    };

    fn sample_http_binding() -> SessionMcpServer {
        SessionMcpServer::Http(SessionMcpHttpServer {
            connection_id: "connection-1".to_string(),
            catalog_entry_id: Some("github".to_string()),
            server_name: "github".to_string(),
            url: "https://api.github.com/mcp?appid=secret".to_string(),
            headers: vec![SessionMcpHeader {
                name: "Authorization".to_string(),
                value: "Bearer secret".to_string(),
            }],
        })
    }

    fn sample_stdio_binding() -> SessionMcpServer {
        SessionMcpServer::Stdio(SessionMcpStdioServer {
            connection_id: "connection-2".to_string(),
            catalog_entry_id: Some("filesystem".to_string()),
            server_name: "filesystem".to_string(),
            command: "mcp-server-filesystem".to_string(),
            args: vec!["/workspace".to_string()],
            env: vec![SessionMcpEnvVar {
                name: "API_KEY".to_string(),
                value: "secret".to_string(),
            }],
        })
    }

    fn sample_cipher() -> DataCipher {
        DataCipher::from_env_value("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=").expect("cipher")
    }

    #[test]
    fn encrypt_and_decrypt_bindings_round_trip() {
        let bindings = vec![sample_http_binding(), sample_stdio_binding()];
        let ciphertext = encrypt_bindings(Some(&sample_cipher()), &bindings)
            .expect("encrypt bindings")
            .expect("ciphertext");

        let restored =
            decrypt_bindings(Some(&sample_cipher()), Some(&ciphertext)).expect("decrypt bindings");

        assert_eq!(restored, bindings);
    }

    #[test]
    fn encrypt_requires_data_key_when_bindings_present() {
        let error =
            encrypt_bindings(None, &[sample_http_binding()]).expect_err("missing key error");
        assert!(matches!(error, SessionMcpBindingsError::MissingDataKey));
    }

    #[test]
    fn decrypt_requires_restart_when_ciphertext_is_corrupt() {
        let error = decrypt_bindings(Some(&sample_cipher()), Some("v1:not-valid-base64"))
            .expect_err("decrypt error");
        assert!(matches!(error, SessionMcpBindingsError::Decrypt(_)));
    }
}

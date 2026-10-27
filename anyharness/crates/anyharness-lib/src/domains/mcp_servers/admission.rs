use std::collections::BTreeMap;

use anyhow::{bail, Context};
use anyharness_contract::v1::mcp_servers::{
    McpServerConfig, McpServerCredential, McpServerTransport,
};

const MAX_SERVER_NAME_BYTES: usize = 100;
const MAX_DESCRIPTION_BYTES: usize = 512;
const MAX_COMMAND_BYTES: usize = 4096;
const MAX_URL_BYTES: usize = 2048;
const MAX_CREDENTIALS: usize = 16;
const MAX_ARGS: usize = 32;
const MAX_ARG_BYTES: usize = 4096;
const MAX_CREDENTIAL_NAME_BYTES: usize = 128;
const MAX_CREDENTIAL_VALUE_BYTES: usize = 65536;

/// Validated admission output. Secrets stay plaintext only inside this
/// structure's lifetime; storage and launch paths seal them.
pub struct AdmittedMcpServer {
    pub server_name: String,
    pub transport: McpServerTransport,
    pub description: Option<String>,
    pub stdio_command: Option<String>,
    pub stdio_args: Vec<String>,
    pub env: Vec<McpServerCredential>,
    pub url: Option<String>,
    pub headers: Vec<McpServerCredential>,
}

fn checked_string(
    value: &str,
    limit: usize,
    what: &str,
    allow_empty: bool,
) -> anyhow::Result<String> {
    if !allow_empty && value.trim().is_empty() {
        bail!("{what} must not be blank");
    }
    if value.len() > limit {
        bail!("{what} is too long");
    }
    if value.chars().any(char::is_control) {
        bail!("{what} must not contain control characters");
    }
    Ok(value.to_owned())
}

fn validate_credential(credential: &McpServerCredential, what: &str) -> anyhow::Result<()> {
    let name = checked_string(
        &credential.name,
        MAX_CREDENTIAL_NAME_BYTES,
        &format!("{what} name"),
        false,
    )?;
    if credential.value.is_empty() {
        bail!("{what} value must not be blank");
    }
    if credential.value.len() > MAX_CREDENTIAL_VALUE_BYTES {
        bail!("{what} value is too long");
    }
    if credential.value.chars().any(char::is_control) {
        bail!("{what} value must not contain control characters");
    }
    if name.chars().any(char::is_whitespace) {
        bail!("{what} name must not contain whitespace");
    }
    Ok(())
}

fn validate_credentials(credentials: &[McpServerCredential], what: &str) -> anyhow::Result<()> {
    if credentials.len() > MAX_CREDENTIALS {
        bail!("A server carries at most {MAX_CREDENTIALS} {what}");
    }
    let mut seen = std::collections::BTreeSet::new();
    for credential in credentials {
        validate_credential(credential, what)?;
        anyhow::ensure!(
            seen.insert(credential.name.clone()),
            "{what} names must be unique"
        );
    }
    Ok(())
}

/// Free but bounded stdio admission (founder ruling): the runtime user may
/// attach any command they type; bounds and the secret-sealing contract apply.
pub fn admit_create(
    server_name: &str,
    description: Option<&str>,
    config: &McpServerConfig,
) -> anyhow::Result<AdmittedMcpServer> {
    let server_name = checked_string(server_name, MAX_SERVER_NAME_BYTES, "Server name", false)?;
    let description = description
        .map(|description| {
            checked_string(description, MAX_DESCRIPTION_BYTES, "Description", false)
        })
        .transpose()?;
    let admitted = match config {
        McpServerConfig::Stdio { command, args, env } => {
            let command = checked_string(command, MAX_COMMAND_BYTES, "Command", false)?;
            if args.len() > MAX_ARGS {
                bail!("A command takes at most {MAX_ARGS} arguments");
            }
            let mut admitted_args = Vec::with_capacity(args.len());
            for argument in args {
                admitted_args.push(checked_string(
                    argument,
                    MAX_ARG_BYTES,
                    "Command argument",
                    true,
                )?);
            }
            validate_credentials(env, "env variable")?;
            AdmittedMcpServer {
                server_name,
                transport: McpServerTransport::Stdio,
                description,
                stdio_command: Some(command),
                stdio_args: admitted_args,
                env: env.clone(),
                url: None,
                headers: Vec::new(),
            }
        }
        McpServerConfig::Http { url, headers } => {
            let url = checked_string(url, MAX_URL_BYTES, "URL", false)?;
            let parsed = url::Url::parse(&url).context("URL is invalid")?;
            let scheme = parsed.scheme();
            anyhow::ensure!(
                scheme == "http" || scheme == "https",
                "URL scheme must be http or https"
            );
            validate_credentials(headers, "header")?;
            AdmittedMcpServer {
                server_name,
                transport: McpServerTransport::Http,
                description,
                stdio_command: None,
                stdio_args: Vec::new(),
                env: Vec::new(),
                url: Some(url),
                headers: headers.clone(),
            }
        }
    };
    Ok(admitted)
}

fn parse_override_map(json: &str) -> anyhow::Result<BTreeMap<String, bool>> {
    if json.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    serde_json::from_str(json).context("Repository overrides are invalid")
}

pub fn parse_repository_overrides(json: &str) -> anyhow::Result<BTreeMap<String, bool>> {
    parse_override_map(json)
}

/// A `user:<server_name>` namespace that is safe in binding summaries and
/// ACP tool names.
pub fn connection_id(server_name: &str) -> String {
    format!("user:{server_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stdio_config() -> McpServerConfig {
        McpServerConfig::Stdio {
            command: "mcp-server-filesystem".to_string(),
            args: vec!["/workspace".to_string()],
            env: Vec::new(),
        }
    }

    #[test]
    fn admits_a_plain_stdio_server() {
        let admitted = admit_create("filesystem", Some("Local files"), &stdio_config())
            .expect("admission succeeds");
        assert_eq!(admitted.transport, McpServerTransport::Stdio);
        assert_eq!(admitted.stdio_args.len(), 1);
    }

    #[test]
    fn server_name_limit_counts_printable_utf8_bytes() {
        let exactly_100_bytes = "é".repeat(50);
        assert_eq!(exactly_100_bytes.len(), MAX_SERVER_NAME_BYTES);
        assert!(admit_create(&exactly_100_bytes, None, &stdio_config()).is_ok());
        assert!(admit_create("space allowed", None, &stdio_config()).is_ok());
        assert!(
            admit_create("tool\u{1}", None, &stdio_config()).is_err(),
            "control characters enforced"
        );
        assert!(
            admit_create(
                &"x".repeat(MAX_SERVER_NAME_BYTES + 1),
                None,
                &stdio_config()
            )
            .is_err()
        );
        assert!(admit_create("tool", None, &stdio_config()).is_ok());
    }

    #[test]
    fn enforces_exact_argument_and_credential_count_bounds() {
        let at_limit = McpServerConfig::Stdio {
            command: "cmd".to_string(),
            args: (0..MAX_ARGS).map(|argument| argument.to_string()).collect(),
            env: Vec::new(),
        };
        assert!(admit_create("tool", None, &at_limit).is_ok());
        let too_many = McpServerConfig::Stdio {
            command: "cmd".to_string(),
            args: (0..MAX_ARGS + 1).map(|argument| argument.to_string()).collect(),
            env: Vec::new(),
        };
        assert!(admit_create("tool", None, &too_many).is_err());

        let credentials = |count: usize| {
            (0..count)
                .map(|index| McpServerCredential {
                    name: format!("CREDENTIAL_{index}"),
                    value: "value".to_string(),
                })
                .collect::<Vec<_>>()
        };
        let env_at_limit = McpServerConfig::Stdio {
            command: "cmd".to_string(),
            args: Vec::new(),
            env: credentials(MAX_CREDENTIALS),
        };
        assert!(admit_create("tool", None, &env_at_limit).is_ok());
        let env_over_limit = McpServerConfig::Stdio {
            command: "cmd".to_string(),
            args: Vec::new(),
            env: credentials(MAX_CREDENTIALS + 1),
        };
        assert!(admit_create("tool", None, &env_over_limit).is_err());

        let headers_at_limit = McpServerConfig::Http {
            url: "https://example.invalid".to_string(),
            headers: credentials(MAX_CREDENTIALS),
        };
        assert!(admit_create("tool", None, &headers_at_limit).is_ok());
        let headers_over_limit = McpServerConfig::Http {
            url: "https://example.invalid".to_string(),
            headers: credentials(MAX_CREDENTIALS + 1),
        };
        assert!(admit_create("tool", None, &headers_over_limit).is_err());
    }

    #[test]
    fn command_and_url_accept_their_exact_byte_limits() {
        let command = McpServerConfig::Stdio {
            command: "x".repeat(MAX_COMMAND_BYTES),
            args: Vec::new(),
            env: Vec::new(),
        };
        assert!(admit_create("tool", None, &command).is_ok());
        let too_long_command = McpServerConfig::Stdio {
            command: "x".repeat(MAX_COMMAND_BYTES + 1),
            args: Vec::new(),
            env: Vec::new(),
        };
        assert!(admit_create("tool", None, &too_long_command).is_err());

        let prefix = "https://example.invalid/";
        let url = format!("{prefix}{}", "x".repeat(MAX_URL_BYTES - prefix.len()));
        assert_eq!(url.len(), MAX_URL_BYTES);
        let http = McpServerConfig::Http {
            url,
            headers: Vec::new(),
        };
        assert!(admit_create("tool", None, &http).is_ok());
        let too_long_url = McpServerConfig::Http {
            url: format!("{prefix}{}", "x".repeat(MAX_URL_BYTES - prefix.len() + 1)),
            headers: Vec::new(),
        };
        assert!(admit_create("tool", None, &too_long_url).is_err());
    }

    #[test]
    fn rejects_control_characters_in_command_arguments_and_credentials() {
        for config in [
            McpServerConfig::Stdio {
                command: "cmd\ninject".to_string(),
                args: Vec::new(),
                env: Vec::new(),
            },
            McpServerConfig::Stdio {
                command: "cmd".to_string(),
                args: vec!["arg\tvalue".to_string()],
                env: Vec::new(),
            },
            McpServerConfig::Stdio {
                command: "cmd".to_string(),
                args: Vec::new(),
                env: vec![McpServerCredential {
                    name: "TOKEN".to_string(),
                    value: "secret\rvalue".to_string(),
                }],
            },
            McpServerConfig::Http {
                url: "https://example.invalid/\n".to_string(),
                headers: Vec::new(),
            },
            McpServerConfig::Http {
                url: "https://example.invalid".to_string(),
                headers: vec![McpServerCredential {
                    name: "Authorization".to_string(),
                    value: "Bearer secret\ninjected".to_string(),
                }],
            },
            McpServerConfig::Http {
                url: "https://example.invalid".to_string(),
                headers: vec![McpServerCredential {
                    name: "X-\u{1}Token".to_string(),
                    value: "secret".to_string(),
                }],
            },
        ] {
            assert!(admit_create("tool", None, &config).is_err());
        }
        assert!(admit_create("tool", Some("description\ninjected"), &stdio_config()).is_err());
        assert!(admit_create("tool\u{1}", None, &stdio_config()).is_err());
    }

    #[test]
    fn rejects_bad_scheme_and_preserves_empty_command_arguments() {
        let ftp = McpServerConfig::Http {
            url: "ftp://example.invalid".to_string(),
            headers: Vec::new(),
        };
        assert!(admit_create("tool", None, &ftp).is_err());
        let empty_arg = McpServerConfig::Stdio {
            command: "cmd".to_string(),
            args: vec![String::new()],
            env: Vec::new(),
        };
        assert_eq!(
            admit_create("tool", None, &empty_arg)
                .expect("an empty argv element is valid")
                .stdio_args,
            vec![String::new()]
        );
    }

    #[test]
    fn rejects_duplicate_and_blank_secrets() {
        let duplicated = McpServerConfig::Http {
            url: "https://example.invalid".to_string(),
            headers: vec![
                McpServerCredential {
                    name: "Authorization".to_string(),
                    value: "a".to_string(),
                },
                McpServerCredential {
                    name: "Authorization".to_string(),
                    value: "b".to_string(),
                },
            ],
        };
        assert!(admit_create("tool", None, &duplicated).is_err());
        let blank = McpServerConfig::Http {
            url: "https://example.invalid".to_string(),
            headers: vec![McpServerCredential {
                name: "Authorization".to_string(),
                value: String::new(),
            }],
        };
        assert!(admit_create("tool", None, &blank).is_err());
    }

    #[test]
    fn connection_namespace_is_summary_safe() {
        assert_eq!(connection_id("filesystem"), "user:filesystem");
    }
}

use super::{model::ProviderInput, LocalProviderError};
use std::collections::BTreeSet;

pub(super) fn validate(input: &mut ProviderInput) -> Result<(), LocalProviderError> {
    let invalid = LocalProviderError::Invalid;
    input.name = input.name.trim().to_owned();
    input.endpoint = input.endpoint.trim().to_owned();
    if input.name.is_empty() || input.name.len() > 255 || input.name.chars().any(char::is_control) {
        return Err(invalid(
            "Use a provider name with 1–255 bytes and no control characters.",
        ));
    }
    let url = reqwest::Url::parse(&input.endpoint)
        .map_err(|_| invalid("Enter an HTTP or HTTPS API base URL."))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.port() == Some(0)
        || !(input.endpoint.starts_with("http://") || input.endpoint.starts_with("https://"))
        || input.endpoint.len() > 4096
        || input
            .endpoint
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '{' | '}' | '\\'))
    {
        return Err(invalid(
            "Use an HTTP or HTTPS base URL without credentials, a query, or a fragment.",
        ));
    }
    let mut seen = BTreeSet::new();
    input.models = input
        .models
        .iter()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty() && seen.insert(id.clone()))
        .collect();
    if input.models.is_empty()
        || input.models.len() > 100
        || input.models.iter().any(|id| {
            id.len() > 1024
                || id
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '{' | '}'))
        })
    {
        return Err(invalid(
            "Enter 1–100 literal model IDs without spaces or braces.",
        ));
    }
    if !matches!(input.api_format.as_str(), "chat" | "responses") {
        return Err(invalid("Choose Chat Completions or Responses."));
    }
    if let Some(key) = &input.api_key {
        if key.len() > 16_384 || key.chars().any(|c| matches!(c, '\r' | '\n' | '\0')) {
            return Err(invalid(
                "Enter an API key without line breaks or null characters.",
            ));
        }
    }
    input.api_key = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_owned);
    let literal = |value: &str| !value.contains("{env:") && !value.contains("{file:");
    if !literal(&input.name)
        || !literal(&input.endpoint)
        || input.api_key.as_deref().is_some_and(|key| !literal(key))
    {
        return Err(invalid(
            "Environment and file substitutions are not supported.",
        ));
    }
    if serde_json::to_vec(input)
        .map_err(|_| LocalProviderError::Unavailable)?
        .len()
        > 65_536
    {
        return Err(invalid("Use at most 64 KiB per provider."));
    }
    Ok(())
}

use std::{net::SocketAddr, time::Duration};

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::{commands::runtime::get_runtime_info, sidecar::SharedSidecar};

use super::keychain;

const PROVIDER_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PROVIDER_KEY_BYTES: usize = 16 * 1024;
const NATIVE_PROVIDER_CREDENTIAL_HEADER: &str = "x-anyharness-native-provider-credential";

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeySaveOutcome {
    Saved,
    Cancelled,
}

trait SecureInputProvider {
    fn prompt(&mut self) -> Result<Option<String>, String>;
}

fn save_key_from_prompt(
    profile_id: &str,
    prompt: &mut impl SecureInputProvider,
    mut save: impl FnMut(&str, &str) -> Result<(), String>,
) -> Result<KeySaveOutcome, String> {
    let Some(mut value) = prompt.prompt()? else {
        return Ok(KeySaveOutcome::Cancelled);
    };
    if value.is_empty() {
        value.zeroize();
        return Err("Provider key cannot be empty.".to_string());
    }

    let result = save(profile_id, &value);
    value.zeroize();
    result.map(|()| KeySaveOutcome::Saved)
}

#[cfg(target_os = "macos")]
fn prompt_for_ai_provider_key(marker: objc2::MainThreadMarker) -> Result<Option<String>, String> {
    use objc2_app_kit::{
        NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSModalResponseCancel, NSSecureTextField,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    let alert = NSAlert::new(marker);
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.setMessageText(&NSString::from_str("Add provider API key"));
    alert.setInformativeText(&NSString::from_str(
        "The key is stored in this Mac’s Keychain and is not exposed to the app’s web content.",
    ));
    let _ = alert.addButtonWithTitle(&NSString::from_str("Save"));
    let _ = alert.addButtonWithTitle(&NSString::from_str("Cancel"));

    let input = NSSecureTextField::new(marker);
    input.setFrame(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(360.0, 24.0),
    ));
    input.setPlaceholderString(Some(&NSString::from_str("API key")));
    alert.setAccessoryView(Some(&input));

    let response = alert.runModal();
    if response == NSAlertFirstButtonReturn {
        let value = input.stringValue().to_string();
        input.setStringValue(&NSString::from_str(""));
        if value.is_empty() {
            return Err("Provider key cannot be empty.".to_string());
        }
        Ok(Some(value))
    } else if response == NSModalResponseCancel {
        input.setStringValue(&NSString::from_str(""));
        Ok(None)
    } else {
        input.setStringValue(&NSString::from_str(""));
        Err("The provider key prompt could not be completed.".to_string())
    }
}

#[cfg(target_os = "macos")]
struct PromptResult(Option<String>);

#[cfg(target_os = "macos")]
impl SecureInputProvider for PromptResult {
    fn prompt(&mut self) -> Result<Option<String>, String> {
        Ok(self.0.take())
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn prompt_ai_provider_key(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<KeySaveOutcome, String> {
    keychain::validate_ai_provider_profile_id(&profile_id)?;

    #[cfg(target_os = "macos")]
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let result = objc2::MainThreadMarker::new()
                .ok_or_else(|| {
                    "The provider key prompt must run on the macOS main thread.".to_string()
                })
                .and_then(prompt_for_ai_provider_key);
            let _ = sender.send(result);
        })
        .map_err(|_| "Could not open the provider key prompt.".to_string())?;
        let prompt_result = receiver
            .recv()
            .map_err(|_| "The provider key prompt did not complete.".to_string())??;
        let mut prompt = PromptResult(prompt_result);
        save_key_from_prompt(&profile_id, &mut prompt, keychain::save_ai_provider_key)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Provider key storage is unsupported on this platform.".to_string())
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn delete_ai_provider_key(profile_id: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        keychain::delete_ai_provider_key(&profile_id)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = profile_id;
        Err("Provider key storage is unsupported on this platform.".to_string())
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn has_ai_provider_key(profile_id: String) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        keychain::has_ai_provider_key(&profile_id)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = profile_id;
        Err("Provider key storage is unsupported on this platform.".to_string())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoverProviderModelsRequest {
    endpoint: String,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn discover_ai_provider_models(
    sidecar: tauri::State<'_, SharedSidecar>,
    profile_id: String,
    endpoint: String,
) -> Result<serde_json::Value, String> {
    keychain::validate_ai_provider_profile_id(&profile_id)?;
    validate_provider_endpoint(&endpoint)?;
    #[cfg(target_os = "macos")]
    let mut credential = keychain::read_ai_provider_key(&profile_id)?;
    #[cfg(not(target_os = "macos"))]
    let mut credential: Option<String> = None;

    if credential.as_ref().is_some_and(|value| value.len() > MAX_PROVIDER_KEY_BYTES) {
        if let Some(value) = credential.as_mut() {
            value.zeroize();
        }
        return Err("Stored provider key exceeds the supported size.".to_string());
    }

    let runtime = get_runtime_info(sidecar).await?;
    if runtime.status != crate::sidecar::RuntimeStatus::Healthy {
        if let Some(value) = credential.as_mut() {
            value.zeroize();
        }
        return Err("The local runtime is not ready.".to_string());
    }
    let url = format!("{}/v1/ai-providers/models", runtime.url.trim_end_matches('/'));
    let response = tokio::time::timeout(PROVIDER_DISCOVERY_TIMEOUT, async {
        let client = reqwest::Client::builder()
            .timeout(PROVIDER_DISCOVERY_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .resolve("localhost", SocketAddr::from(([127, 0, 0, 1], runtime.port)))
            .resolve("127.0.0.1", SocketAddr::from(([127, 0, 0, 1], runtime.port)))
            .build()
            .map_err(|_| "Could not prepare local provider discovery.".to_string())?;
        let mut request = client
            .post(url)
            .json(&DiscoverProviderModelsRequest { endpoint });
        if let Some(value) = credential.as_deref() {
            request = request.header(NATIVE_PROVIDER_CREDENTIAL_HEADER, value);
        }
        request
            .send()
            .await
            .map_err(|_| "Could not reach local provider discovery.".to_string())?
            .error_for_status()
            .map_err(|response| map_discovery_status(response.status()))?
            .json::<serde_json::Value>()
            .await
            .map_err(|_| "The local runtime returned an invalid model list.".to_string())
    })
    .await
    .map_err(|_| "Provider model discovery timed out.".to_string())?;

    if let Some(value) = credential.as_mut() {
        value.zeroize();
    }
    response
}

fn validate_provider_endpoint(endpoint: &str) -> Result<(), String> {
    let parsed = url::Url::parse(endpoint.trim())
        .map_err(|_| "Enter a valid provider endpoint URL.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none_or(str::is_empty)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || endpoint
            .split_once("://")
            .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
            .is_some_and(|authority| authority.contains('@'))
        || parsed.fragment().is_some()
    {
        return Err("Provider endpoints must be HTTP(S), contain no credentials, and have no fragment.".to_string());
    }
    Ok(())
}

fn map_discovery_status(status: reqwest::StatusCode) -> String {
    let code = status.as_u16();
    match code {
        400 => "The provider endpoint is invalid.".to_string(),
        401 => "The provider rejected its key.".to_string(),
        502 => "The provider returned an unusable model list.".to_string(),
        503 => "The provider could not be reached.".to_string(),
        504 => "Provider model discovery timed out.".to_string(),
        _ => "Provider model discovery failed.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const PROFILE_A: &str = "0f1e2d3c-4b5a-6978-8f0e-1d2c3b4a5968";

    #[derive(Default)]
    struct FakeKeychain {
        entries: HashMap<String, String>,
        writes: usize,
    }

    impl keychain::AiProviderKeyBackend for FakeKeychain {
        fn set(&mut self, account: &str, value: &str) -> Result<(), String> {
            self.writes += 1;
            self.entries.insert(account.to_string(), value.to_string());
            Ok(())
        }

        fn delete(&mut self, account: &str) -> Result<(), String> {
            self.entries.remove(account);
            Ok(())
        }
    }

    struct FakeSecureInput(Option<String>);

    impl SecureInputProvider for FakeSecureInput {
        fn prompt(&mut self) -> Result<Option<String>, String> {
            Ok(self.0.take())
        }
    }

    #[test]
    fn provider_endpoint_validation_rejects_userinfo_and_fragments_without_echoing_input() {
        assert!(validate_provider_endpoint("http://localhost:1234/v1").is_ok());
        for endpoint in [
            "http://user:pass@localhost/v1?token=secret",
            "http://@localhost/v1",
            "http://:pass@localhost/v1",
            "http://localhost/v1#fragment-secret",
        ] {
            let error = validate_provider_endpoint(endpoint).unwrap_err();
            assert!(!error.contains("secret"));
        }
    }

    #[test]
    fn provider_secure_prompt_saves_key_and_returns_only_status() {
        let mut prompt = FakeSecureInput(Some("secret-value".to_string()));
        let mut keychain = FakeKeychain::default();

        let outcome = save_key_from_prompt(PROFILE_A, &mut prompt, |profile_id, value| {
            keychain::save_ai_provider_key_with_backend(profile_id, value, &mut keychain)
        })
        .expect("saving through secure input should work");

        assert_eq!(outcome, KeySaveOutcome::Saved);
        assert_eq!(
            keychain
                .entries
                .get("profile:0f1e2d3c-4b5a-6978-8f0e-1d2c3b4a5968"),
            Some(&"secret-value".to_string())
        );
        assert_eq!(keychain.writes, 1);
        assert_eq!(
            serde_json::to_string(&outcome).expect("status should serialize"),
            "\"saved\""
        );
    }

    #[test]
    fn provider_secure_prompt_cancellation_writes_nothing() {
        let mut prompt = FakeSecureInput(None);
        let mut keychain = FakeKeychain::default();

        let outcome = save_key_from_prompt(PROFILE_A, &mut prompt, |profile_id, value| {
            keychain::save_ai_provider_key_with_backend(profile_id, value, &mut keychain)
        })
        .expect("cancellation should be handled");

        assert_eq!(outcome, KeySaveOutcome::Cancelled);
        assert!(keychain.entries.is_empty());
        assert_eq!(keychain.writes, 0);
        assert_eq!(
            serde_json::to_string(&outcome).expect("status should serialize"),
            "\"cancelled\""
        );
    }
}

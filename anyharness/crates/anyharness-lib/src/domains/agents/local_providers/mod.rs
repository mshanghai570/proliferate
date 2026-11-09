pub mod model;
mod store;
#[cfg(test)]
mod tests;
mod validation;

use super::route_auth::profile::{HarnessSources, ProviderConfigProfile, ResolvedSource};
use super::route_auth::{self, AgentRuntimeAuthProfile, RouteAuthError};
use crate::persistence::data_cipher::{
    decrypt_bytes, encrypt_bytes, load_data_cipher_from_env, DataCipher,
};
use model::{ProviderInput, ProviderInventory, ProviderSummary, SealedProvider};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum LocalProviderError {
    #[error("Invalid provider settings. {0}")]
    Invalid(&'static str),
    #[error("Provider was not found.")]
    NotFound,
    #[error("Restart the desktop app to restore its encryption key.")]
    MissingKey,
    #[error("Managed authentication is active. Local connections are paused while that profile is applied.")]
    ManagedAuth,
    #[error("Local provider settings could not be accessed.")]
    Unavailable,
}

pub struct LocalProviderManager {
    home: PathBuf,
    cipher: Option<DataCipher>,
}
impl LocalProviderManager {
    pub fn new(home: PathBuf, cipher: Option<DataCipher>) -> Self {
        Self { home, cipher }
    }
    pub fn from_home(home: &Path) -> Result<Self, LocalProviderError> {
        Ok(Self::new(
            home.to_owned(),
            load_data_cipher_from_env().map_err(|_| LocalProviderError::Unavailable)?,
        ))
    }
    pub fn managed_auth_active(&self) -> Result<bool, LocalProviderError> {
        route_auth::load_effective_state(&self.home, route_auth::current_server_origin().as_deref())
            .map(|state| state.is_some())
            .map_err(|_| LocalProviderError::Unavailable)
    }
    pub fn list(&self) -> Result<ProviderInventory, LocalProviderError> {
        let _lock = store::lock(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let catalog = store::read(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        Ok(ProviderInventory {
            providers: catalog.providers.iter().map(summary).collect(),
            encryption_available: self.cipher.is_some(),
            managed_auth_active: self.managed_auth_active()?,
        })
    }
    pub fn create(&self, mut input: ProviderInput) -> Result<ProviderSummary, LocalProviderError> {
        validation::validate(&mut input)?;
        if self.managed_auth_active()? {
            return Err(LocalProviderError::ManagedAuth);
        }
        let cipher = self.cipher.as_ref().ok_or(LocalProviderError::MissingKey)?;
        let _lock = store::lock(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let mut catalog = store::read(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        if catalog.providers.len() >= store::MAX_PROVIDERS {
            return Err(LocalProviderError::Invalid(
                "Remove a connection before adding more than 32 providers.",
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let bytes = serde_json::to_vec(&SealedProvider {
            id: id.clone(),
            input: input.clone(),
        })
        .map_err(|_| LocalProviderError::Unavailable)?;
        let ciphertext =
            encrypt_bytes(cipher, &bytes).map_err(|_| LocalProviderError::Unavailable)?;
        let row = store::Record {
            id,
            name: input.name,
            enabled: true,
            model_count: input.models.len(),
            ciphertext,
        };
        let result = summary(&row);
        catalog.providers.push(row);
        store::write(&self.home, &mut catalog).map_err(|_| LocalProviderError::Unavailable)?;
        changed("create");
        Ok(result)
    }
    pub fn set_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<ProviderSummary, LocalProviderError> {
        if enabled && self.managed_auth_active()? {
            return Err(LocalProviderError::ManagedAuth);
        }
        let _lock = store::lock(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let mut catalog = store::read(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let row = catalog
            .providers
            .iter_mut()
            .find(|row| row.id == id)
            .ok_or(LocalProviderError::NotFound)?;
        if enabled {
            self.decrypt(row)?;
        }
        row.enabled = enabled;
        let result = summary(row);
        store::write(&self.home, &mut catalog).map_err(|_| LocalProviderError::Unavailable)?;
        changed("toggle");
        Ok(result)
    }
    pub fn remove(&self, id: &str) -> Result<(), LocalProviderError> {
        let _lock = store::lock(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let mut catalog = store::read(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        if !catalog.providers.iter().any(|row| row.id == id) {
            return Err(LocalProviderError::NotFound);
        }
        catalog.providers.retain(|row| row.id != id);
        store::write(&self.home, &mut catalog).map_err(|_| LocalProviderError::Unavailable)?;
        changed("remove");
        Ok(())
    }
    fn decrypt(&self, row: &store::Record) -> Result<ProviderInput, LocalProviderError> {
        let cipher = self.cipher.as_ref().ok_or(LocalProviderError::MissingKey)?;
        let bytes =
            decrypt_bytes(cipher, &row.ciphertext).map_err(|_| LocalProviderError::Unavailable)?;
        let mut sealed: SealedProvider =
            serde_json::from_slice(&bytes).map_err(|_| LocalProviderError::Unavailable)?;
        if sealed.id != row.id
            || sealed.input.name != row.name
            || sealed.input.models.len() != row.model_count
        {
            return Err(LocalProviderError::Unavailable);
        }
        validation::validate(&mut sealed.input)?;
        Ok(sealed.input)
    }
    fn profile(&self) -> Result<Option<AgentRuntimeAuthProfile>, LocalProviderError> {
        let _lock = store::lock(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let catalog = store::read(&self.home).map_err(|_| LocalProviderError::Unavailable)?;
        let mut sources = Vec::new();
        for row in catalog.providers.iter().filter(|row| row.enabled) {
            let input = self.decrypt(row)?;
            let provider_id = format!("custom-local-{}", row.id);
            let mut env = BTreeMap::new();
            let mut options = json!({ "baseURL": input.endpoint, "apiKey": "" });
            if let Some(key) = input.api_key {
                let key_name = format!(
                    "PROLIFERATE_CUSTOM_LOCAL_{}_KEY",
                    row.id.replace('-', "_").to_uppercase()
                );
                options["apiKey"] = json!(format!("{{env:{key_name}}}"));
                env.insert(key_name, key);
            }
            let models: BTreeMap<_, _> = input
                .models
                .into_iter()
                .map(|id| (id.clone(), json!({ "name": id })))
                .collect();
            env.insert("OPENCODE_CONFIG_CONTENT".into(), json!({"provider": { (provider_id): {
                "name": input.name, "npm": if input.api_format == "responses" { "@ai-sdk/openai" } else { "@ai-sdk/openai-compatible" },
                "options": options, "models": models
            }}}).to_string());
            sources.push(ResolvedSource::ProviderConfig(ProviderConfigProfile {
                config_kind: "openai_compatible".into(),
                env,
            }));
        }
        if sources.is_empty() {
            return Ok(None);
        }
        Ok(Some(AgentRuntimeAuthProfile::Sources(HarnessSources {
            harness_kind: "opencode".into(),
            revision: -catalog.revision,
            sources,
        })))
    }
}

fn summary(row: &store::Record) -> ProviderSummary {
    ProviderSummary {
        id: row.id.clone(),
        name: row.name.clone(),
        enabled: row.enabled,
        model_count: row.model_count,
    }
}
fn changed(action: &'static str) {
    tracing::info!(
        event = "anyharness.local_provider.changed",
        action,
        "Local provider settings changed"
    );
}

pub(super) fn launch_profile(
    home: &Path,
) -> Result<Option<AgentRuntimeAuthProfile>, RouteAuthError> {
    LocalProviderManager::from_home(home).and_then(|manager| manager.profile()).map_err(|_| RouteAuthError::SelectionIncomplete {
        harness_kind: "opencode".into(), detail: "Local provider settings could not be decrypted or read. Check Settings → Agents → LLM providers.".into(),
    })
}

pub(crate) fn basis_revision(home: &Path) -> String {
    use sha2::{Digest, Sha256};
    match store::read(home).and_then(|catalog| Ok(serde_json::to_vec(&catalog)?)) {
        Ok(bytes) => format!("{:x}", Sha256::digest(bytes)),
        Err(_) => "local-provider-vault-unavailable".into(),
    }
}

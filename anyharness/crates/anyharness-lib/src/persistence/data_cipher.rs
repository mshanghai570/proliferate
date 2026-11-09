use aes_gcm_siv::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm_siv::Aes256GcmSiv;
use anyhow::Context;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;

pub const DATA_KEY_ENV_VAR: &str = "ANYHARNESS_DATA_KEY";
const CIPHERTEXT_PREFIX: &str = "v1:";
const NONCE_LEN: usize = 12;

#[derive(Clone)]
pub struct DataCipher {
    key_bytes: [u8; 32],
}

impl DataCipher {
    pub fn from_env_value(value: &str) -> Result<Self, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("must not be blank".to_string());
        }

        let decoded = STANDARD
            .decode(trimmed)
            .or_else(|_| URL_SAFE_NO_PAD.decode(trimmed))
            .map_err(|error| format!("must be valid base64: {error}"))?;
        let key_bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_| "must decode to exactly 32 bytes".to_string())?;

        Ok(Self { key_bytes })
    }

    fn algorithm(&self) -> Aes256GcmSiv {
        Aes256GcmSiv::new_from_slice(&self.key_bytes).expect("32-byte key")
    }
}

pub fn load_data_cipher_from_env() -> Result<Option<DataCipher>, String> {
    let Some(value) = std::env::var(DATA_KEY_ENV_VAR).ok() else {
        return Ok(None);
    };
    DataCipher::from_env_value(&value).map(Some)
}

pub fn encrypt_bytes(cipher: &DataCipher, plaintext: &[u8]) -> anyhow::Result<String> {
    let nonce = Aes256GcmSiv::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .algorithm()
        .encrypt(&nonce, plaintext)
        .map_err(|error| anyhow::anyhow!("encrypt data: {error}"))?;

    let mut encoded = nonce.to_vec();
    encoded.extend(ciphertext);
    Ok(format!("{CIPHERTEXT_PREFIX}{}", STANDARD.encode(encoded)))
}

pub fn decrypt_bytes(cipher: &DataCipher, ciphertext: &str) -> anyhow::Result<Vec<u8>> {
    let encoded = ciphertext
        .strip_prefix(CIPHERTEXT_PREFIX)
        .ok_or_else(|| anyhow::anyhow!("unsupported ciphertext version"))?;
    let decoded = STANDARD
        .decode(encoded)
        .context("decode ciphertext payload")?;
    if decoded.len() <= NONCE_LEN {
        anyhow::bail!("ciphertext payload missing nonce");
    }

    let (nonce_bytes, encrypted) = decoded.split_at(NONCE_LEN);
    cipher
        .algorithm()
        .decrypt(nonce_bytes.into(), encrypted)
        .map_err(|error| anyhow::anyhow!("decrypt data: {error}"))
}

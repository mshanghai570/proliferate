use std::fmt;

use zeroize::Zeroize;

pub(crate) mod models;

pub(crate) struct ProviderSecret(String);

impl ProviderSecret {
    pub(crate) fn new(secret: String) -> Self {
        Self(secret)
    }

    pub(super) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ProviderSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderSecret([REDACTED])")
    }
}

impl Drop for ProviderSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

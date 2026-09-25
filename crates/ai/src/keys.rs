//! API keys, kept in Windows Credential Manager only (service "Fazasanj", account =
//! provider id). Keys never go to SQLite, logs, the UI or error messages.

use std::sync::{Arc, OnceLock};

use fazasanj_model::AiProvider;
use keyring_core::{CredentialStore, Error as KeyringError};

use crate::models::provider_id;
use crate::AiError;

pub const SERVICE: &str = "Fazasanj";

/// An API key. Debug is redacted and there is no Display or Serialize on purpose.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// Surrounding whitespace is dropped (keys are often pasted with a trailing newline).
    pub fn new(key: impl Into<String>) -> Self {
        let key: String = key.into();
        ApiKey(key.trim().to_owned())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

/// Keyring errors are mapped by kind only. Some variants carry the raw secret bytes, so they
/// are never formatted.
fn store_error(e: &KeyringError) -> AiError {
    let kind = match e {
        KeyringError::PlatformFailure(_) => "platform_failure",
        KeyringError::NoStorageAccess(_) => "no_storage_access",
        KeyringError::NoEntry => "no_entry",
        KeyringError::BadEncoding(_) | KeyringError::BadDataFormat(..) => "bad_encoding",
        KeyringError::TooLong(..) => "too_long",
        KeyringError::Invalid(..) => "invalid",
        KeyringError::NoDefaultStore => "unavailable",
        _ => "other",
    };
    AiError::KeyStore(kind.to_owned())
}

/// A credential store plus service name. The app uses the Windows one; tests use the
/// in-memory mock from keyring-core.
pub(crate) struct Vault {
    store: Arc<CredentialStore>,
    service: String,
}

impl Vault {
    pub(crate) fn new(store: Arc<CredentialStore>, service: &str) -> Self {
        Vault {
            store,
            service: service.to_owned(),
        }
    }

    fn entry(&self, provider: AiProvider) -> Result<keyring_core::Entry, AiError> {
        self.store
            .build(&self.service, provider_id(provider), None)
            .map_err(|e| store_error(&e))
    }

    pub(crate) fn set(&self, provider: AiProvider, key: &ApiKey) -> Result<(), AiError> {
        if key.is_empty() {
            return Err(AiError::NoKey);
        }
        self.entry(provider)?
            .set_password(key.expose())
            .map_err(|e| store_error(&e))
    }

    pub(crate) fn get(&self, provider: AiProvider) -> Result<Option<ApiKey>, AiError> {
        match self.entry(provider)?.get_password() {
            Ok(k) => Ok(Some(ApiKey::new(k)).filter(|k| !k.is_empty())),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(store_error(&e)),
        }
    }

    pub(crate) fn delete(&self, provider: AiProvider) -> Result<(), AiError> {
        match self.entry(provider)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(store_error(&e)),
        }
    }
}

fn native() -> Result<&'static Vault, AiError> {
    static VAULT: OnceLock<Option<Vault>> = OnceLock::new();
    VAULT
        .get_or_init(native_store)
        .as_ref()
        .ok_or_else(|| AiError::KeyStore("unavailable".to_owned()))
}

#[cfg(windows)]
fn native_store() -> Option<Vault> {
    let store: Arc<CredentialStore> = windows_native_keyring_store::Store::new().ok()?;
    Some(Vault::new(store, SERVICE))
}

#[cfg(not(windows))]
fn native_store() -> Option<Vault> {
    None
}

pub fn set(provider: AiProvider, key: &str) -> Result<(), AiError> {
    native()?.set(provider, &ApiKey::new(key))
}

pub fn get(provider: AiProvider) -> Result<Option<ApiKey>, AiError> {
    native()?.get(provider)
}

/// Deleting a key that does not exist is fine.
pub fn delete(provider: AiProvider) -> Result<(), AiError> {
    native()?.delete(provider)
}

pub fn has(provider: AiProvider) -> bool {
    matches!(get(provider), Ok(Some(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_vault() -> Vault {
        let store: Arc<CredentialStore> = keyring_core::mock::Store::new().unwrap();
        Vault::new(store, "Fazasanj-test")
    }

    #[test]
    fn debug_is_redacted() {
        let k = ApiKey::new("  sk-secret-value\n");
        assert_eq!(format!("{k:?}"), "ApiKey(***)");
        assert_eq!(format!("{:?}", Some(&k)), "Some(ApiKey(***))");
        assert_eq!(k.expose(), "sk-secret-value");
    }

    #[test]
    fn vault_roundtrip() {
        let v = mock_vault();
        assert!(v.get(AiProvider::Groq).unwrap().is_none());
        v.set(AiProvider::Groq, &ApiKey::new("gsk_abc")).unwrap();
        assert_eq!(
            v.get(AiProvider::Groq).unwrap().unwrap().expose(),
            "gsk_abc"
        );
        v.delete(AiProvider::Groq).unwrap();
        assert!(v.get(AiProvider::Groq).unwrap().is_none());
        v.delete(AiProvider::Groq).unwrap();
        assert_eq!(
            v.set(AiProvider::Groq, &ApiKey::new("  ")),
            Err(AiError::NoKey)
        );
    }

    /// Touches the real Credential Manager under a test service name and cleans up after.
    #[cfg(windows)]
    #[test]
    fn windows_credential_manager_roundtrip() {
        let store: Arc<CredentialStore> = windows_native_keyring_store::Store::new().unwrap();
        let v = Vault::new(store, "Fazasanj-selftest");
        v.set(AiProvider::Gemini, &ApiKey::new("test-key-123"))
            .unwrap();
        let got = v.get(AiProvider::Gemini).unwrap();
        v.delete(AiProvider::Gemini).unwrap();
        assert_eq!(got.unwrap().expose(), "test-key-123");
        assert!(v.get(AiProvider::Gemini).unwrap().is_none());
    }
}

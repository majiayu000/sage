//! Credential backend abstraction.

use super::source::CredentialSource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Backend category used for status and recovery messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialBackendKind {
    SecureStore,
    OAuth,
    Fake,
    Unsupported,
}

impl CredentialBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SecureStore => "secure_store",
            Self::OAuth => "oauth",
            Self::Fake => "fake",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Stored credential plus audit source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecord {
    pub provider: String,
    pub secret: String,
    pub source: CredentialSource,
    pub backend: CredentialBackendKind,
}

impl CredentialRecord {
    pub fn new(
        provider: impl Into<String>,
        secret: impl Into<String>,
        backend: CredentialBackendKind,
    ) -> Self {
        let provider = provider.into();
        let source = match backend {
            CredentialBackendKind::OAuth => CredentialSource::oauth(provider.clone()),
            _ => CredentialSource::keychain(backend.as_str()),
        };
        Self {
            provider,
            secret: secret.into(),
            source,
            backend,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialBackendErrorKind {
    Unsupported,
    NotFound,
    SaveFailed,
    LogoutFailed,
    RevokeFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialBackendError {
    pub kind: CredentialBackendErrorKind,
    pub provider: String,
    pub backend: CredentialBackendKind,
    pub message: String,
    pub recoverable: bool,
}

impl CredentialBackendError {
    pub fn new(
        kind: CredentialBackendErrorKind,
        provider: impl Into<String>,
        backend: CredentialBackendKind,
        message: impl Into<String>,
        recoverable: bool,
    ) -> Self {
        Self {
            kind,
            provider: provider.into(),
            backend,
            message: message.into(),
            recoverable,
        }
    }
}

/// Durable credential storage. Implementations must not silently downgrade to plaintext.
pub trait CredentialBackend: Send + Sync {
    fn kind(&self) -> CredentialBackendKind;
    fn load(&self, provider: &str) -> Result<Option<CredentialRecord>, CredentialBackendError>;
    fn save(&self, provider: &str, secret: &str) -> Result<(), CredentialBackendError>;
    fn logout(&self, provider: &str) -> Result<(), CredentialBackendError>;
    fn revoke(&self, provider: &str) -> Result<(), CredentialBackendError>;
}

#[derive(Debug, Default)]
pub struct UnsupportedCredentialBackend;

impl CredentialBackend for UnsupportedCredentialBackend {
    fn kind(&self) -> CredentialBackendKind {
        CredentialBackendKind::Unsupported
    }

    fn load(&self, _provider: &str) -> Result<Option<CredentialRecord>, CredentialBackendError> {
        Ok(None)
    }

    fn save(&self, provider: &str, _secret: &str) -> Result<(), CredentialBackendError> {
        Err(unsupported(provider))
    }

    fn logout(&self, provider: &str) -> Result<(), CredentialBackendError> {
        Err(unsupported(provider))
    }

    fn revoke(&self, provider: &str) -> Result<(), CredentialBackendError> {
        Err(unsupported(provider))
    }
}

fn unsupported(provider: &str) -> CredentialBackendError {
    CredentialBackendError::new(
        CredentialBackendErrorKind::Unsupported,
        provider,
        CredentialBackendKind::Unsupported,
        "secure credential backend is unavailable",
        true,
    )
}

#[derive(Debug, Default)]
pub struct FakeCredentialBackend {
    records: Mutex<HashMap<String, String>>,
    fail_revoke: Mutex<bool>,
}

impl FakeCredentialBackend {
    pub fn with_record(provider: &str, secret: &str) -> Self {
        let backend = Self::default();
        backend
            .records
            .lock()
            .expect("fake backend mutex")
            .insert(provider.to_string(), secret.to_string());
        backend
    }

    pub fn fail_revoke(&self) {
        *self.fail_revoke.lock().expect("fake backend mutex") = true;
    }
}

impl CredentialBackend for FakeCredentialBackend {
    fn kind(&self) -> CredentialBackendKind {
        CredentialBackendKind::Fake
    }

    fn load(&self, provider: &str) -> Result<Option<CredentialRecord>, CredentialBackendError> {
        Ok(self
            .records
            .lock()
            .expect("fake backend mutex")
            .get(provider)
            .cloned()
            .map(|secret| CredentialRecord::new(provider, secret, CredentialBackendKind::Fake)))
    }

    fn save(&self, provider: &str, secret: &str) -> Result<(), CredentialBackendError> {
        self.records
            .lock()
            .expect("fake backend mutex")
            .insert(provider.to_string(), secret.to_string());
        Ok(())
    }

    fn logout(&self, provider: &str) -> Result<(), CredentialBackendError> {
        self.records
            .lock()
            .expect("fake backend mutex")
            .remove(provider);
        Ok(())
    }

    fn revoke(&self, provider: &str) -> Result<(), CredentialBackendError> {
        if *self.fail_revoke.lock().expect("fake backend mutex") {
            return Err(CredentialBackendError::new(
                CredentialBackendErrorKind::RevokeFailed,
                provider,
                CredentialBackendKind::Fake,
                "fake revoke failed",
                true,
            ));
        }
        self.logout(provider)
    }
}

//! Collection of resolved credentials for multiple providers

use super::credential::ResolvedCredential;

/// A collection of resolved credentials for multiple providers
#[derive(Debug, Clone, Default)]
pub struct ResolvedCredentials {
    credentials: Vec<ResolvedCredential>,
}

impl ResolvedCredentials {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a resolved credential
    pub fn add(&mut self, credential: ResolvedCredential) {
        // Replace existing credential for same provider if new one has higher priority
        if let Some(existing) = self
            .credentials
            .iter_mut()
            .find(|c| c.provider == credential.provider)
        {
            if credential.priority() < existing.priority() {
                *existing = credential;
            }
        } else {
            self.credentials.push(credential);
        }
    }

    /// Get a credential for a specific provider
    pub fn get(&self, provider: &str) -> Option<&ResolvedCredential> {
        self.credentials.iter().find(|c| c.provider == provider)
    }

    /// Get the API key for a provider
    pub fn get_api_key(&self, provider: &str) -> Option<&str> {
        self.get(provider).and_then(|c| c.value())
    }

    /// Check if any credentials are configured
    pub fn has_any(&self) -> bool {
        self.credentials.iter().any(|c| c.has_value())
    }

    /// Get all configured providers
    pub fn configured_providers(&self) -> Vec<&str> {
        self.credentials
            .iter()
            .filter(|c| c.has_value())
            .map(|c| c.provider.as_str())
            .collect()
    }

    /// Get all missing providers
    pub fn missing_providers(&self) -> Vec<&str> {
        self.credentials
            .iter()
            .filter(|c| c.is_missing())
            .map(|c| c.provider.as_str())
            .collect()
    }

    /// Iterate over all credentials
    pub fn iter(&self) -> impl Iterator<Item = &ResolvedCredential> {
        self.credentials.iter()
    }

    /// Get the number of credentials
    pub fn len(&self) -> usize {
        self.credentials.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.credentials.is_empty()
    }
}

impl IntoIterator for ResolvedCredentials {
    type Item = ResolvedCredential;
    type IntoIter = std::vec::IntoIter<ResolvedCredential>;

    fn into_iter(self) -> Self::IntoIter {
        self.credentials.into_iter()
    }
}

impl<'a> IntoIterator for &'a ResolvedCredentials {
    type Item = &'a ResolvedCredential;
    type IntoIter = std::slice::Iter<'a, ResolvedCredential>;

    fn into_iter(self) -> Self::IntoIter {
        self.credentials.iter()
    }
}

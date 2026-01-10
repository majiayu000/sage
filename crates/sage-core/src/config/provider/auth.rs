//! Authentication configuration

use serde::{Deserialize, Serialize};

/// Authentication configuration for API access
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiAuthConfig {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Organization ID (used by OpenAI for billing/access control)
    pub organization: Option<String>,
    /// Project ID (used by some providers for project-level access)
    pub project_id: Option<String>,
}

impl ApiAuthConfig {
    /// Create a new authentication config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set organization ID
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Set project ID
    pub fn with_project_id(mut self, project: impl Into<String>) -> Self {
        self.project_id = Some(project.into());
        self
    }
}

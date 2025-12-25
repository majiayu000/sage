//! UI, Workspace, and Model configuration settings

use serde::{Deserialize, Serialize};

/// UI settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiSettings {
    /// Show progress indicators
    #[serde(default)]
    pub show_progress: Option<bool>,

    /// Theme (light/dark/auto)
    #[serde(default)]
    pub theme: Option<String>,

    /// Enable colors
    #[serde(default)]
    pub colors: Option<bool>,

    /// Verbose output
    #[serde(default)]
    pub verbose: Option<bool>,

    /// Maximum output width
    #[serde(default)]
    pub max_width: Option<usize>,
}

impl UiSettings {
    /// Merge another UI settings
    pub fn merge(&mut self, other: UiSettings) {
        if other.show_progress.is_some() {
            self.show_progress = other.show_progress;
        }
        if other.theme.is_some() {
            self.theme = other.theme;
        }
        if other.colors.is_some() {
            self.colors = other.colors;
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.max_width.is_some() {
            self.max_width = other.max_width;
        }
    }
}

/// Workspace settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Files/directories to ignore
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Include patterns
    #[serde(default)]
    pub include: Vec<String>,

    /// Working directory override
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Project type hint
    #[serde(default)]
    pub project_type: Option<String>,
}

impl WorkspaceSettings {
    /// Merge another workspace settings
    pub fn merge(&mut self, other: WorkspaceSettings) {
        self.ignore.extend(other.ignore);
        self.include.extend(other.include);
        if other.working_directory.is_some() {
            self.working_directory = other.working_directory;
        }
        if other.project_type.is_some() {
            self.project_type = other.project_type;
        }
    }
}

/// Model settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelSettings {
    /// Default model to use
    #[serde(default)]
    pub default_model: Option<String>,

    /// Maximum tokens
    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// Temperature
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Provider override
    #[serde(default)]
    pub provider: Option<String>,

    /// API base URL override
    #[serde(default)]
    pub api_base: Option<String>,
}

impl ModelSettings {
    /// Merge another model settings
    pub fn merge(&mut self, other: ModelSettings) {
        if other.default_model.is_some() {
            self.default_model = other.default_model;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.temperature.is_some() {
            self.temperature = other.temperature;
        }
        if other.provider.is_some() {
            self.provider = other.provider;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
    }
}

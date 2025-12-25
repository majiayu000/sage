//! File handling operations for plan mode

use std::path::PathBuf;

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{SageError, SageResult};

use super::core::ModeManager;

impl ModeManager {
    /// Generate a plan file path
    pub(super) fn generate_plan_path(&self, name: Option<&str>) -> PathBuf {
        let generated_name;
        let name = match name {
            Some(n) => n,
            None => {
                // Generate a unique name
                generated_name = uuid::Uuid::new_v4().to_string();
                &generated_name[..8]
            }
        };

        // Create a descriptive name
        let adjectives = ["ancient", "bright", "cosmic", "dancing", "elegant"];
        let nouns = ["river", "mountain", "forest", "ocean", "meadow"];

        let idx1 = name.bytes().next().unwrap_or(0) as usize % adjectives.len();
        let idx2 = name.bytes().last().unwrap_or(0) as usize % nouns.len();

        let descriptive = format!(
            "{}-{}-{}",
            adjectives[idx1],
            nouns[idx2],
            &name[..4.min(name.len())]
        );

        self.plan_dir.join(format!("{}.md", descriptive))
    }

    /// Save plan content
    pub async fn save_plan(&self, content: &str) -> SageResult<PathBuf> {
        let state = self.state.read().await;

        let plan_file = state
            .plan_config
            .as_ref()
            .and_then(|c| c.plan_file.clone())
            .ok_or_else(|| SageError::invalid_input("No plan file configured".to_string()))?;

        drop(state);

        // Ensure directory exists
        if let Some(parent) = plan_file.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::storage(format!("Failed to create plan directory: {}", e))
            })?;
        }

        // Write content
        let mut file = fs::File::create(&plan_file)
            .await
            .map_err(|e| SageError::storage(format!("Failed to create plan file: {}", e)))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| SageError::storage(format!("Failed to write plan file: {}", e)))?;

        tracing::info!("Saved plan to {:?}", plan_file);
        Ok(plan_file)
    }

    /// Load plan content
    pub async fn load_plan(&self) -> SageResult<Option<String>> {
        let state = self.state.read().await;

        let plan_file = match state.plan_config.as_ref().and_then(|c| c.plan_file.clone()) {
            Some(f) => f,
            None => return Ok(None),
        };

        drop(state);

        if !plan_file.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(&plan_file)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open plan file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read plan file: {}", e)))?;

        Ok(Some(content))
    }
}

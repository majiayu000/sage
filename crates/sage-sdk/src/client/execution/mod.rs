//! Task execution module

mod run;
mod unified;

use sage_core::{config::model::Config, skills::SkillRegistry, tools::Tool};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(super) fn resolve_working_directory(requested: Option<PathBuf>, config: &Config) -> PathBuf {
    requested
        .or_else(|| config.working_directory.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

#[cfg(feature = "default-tools")]
pub(super) fn default_tools(
    working_directory: impl Into<PathBuf>,
    skill_registry: Arc<RwLock<SkillRegistry>>,
) -> Vec<Arc<dyn Tool>> {
    sage_tools::get_default_tools_with_context(working_directory, skill_registry)
}

#[cfg(not(feature = "default-tools"))]
pub(super) fn default_tools(
    _working_directory: impl Into<PathBuf>,
    _skill_registry: Arc<RwLock<SkillRegistry>>,
) -> Vec<Arc<dyn Tool>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sage_core::tools::types::ToolCall;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, params: serde_json::Value) -> ToolCall {
        let mut arguments = HashMap::new();
        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                arguments.insert(key.clone(), value.clone());
            }
        }

        ToolCall::new(id, name, arguments)
    }

    #[test]
    fn resolve_working_directory_prefers_requested_directory()
    -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = tempfile::tempdir()?;
        let requested_dir = tempfile::tempdir()?;
        let config = Config {
            working_directory: Some(config_dir.path().to_path_buf()),
            ..Default::default()
        };

        let resolved = resolve_working_directory(Some(requested_dir.path().to_path_buf()), &config);

        assert_eq!(resolved, requested_dir.path());
        Ok(())
    }

    #[test]
    fn resolve_working_directory_uses_config_directory() -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = tempfile::tempdir()?;
        let config = Config {
            working_directory: Some(config_dir.path().to_path_buf()),
            ..Default::default()
        };

        let resolved = resolve_working_directory(None, &config);

        assert_eq!(resolved, config_dir.path());
        Ok(())
    }

    #[cfg(feature = "default-tools")]
    #[tokio::test]
    async fn default_tools_use_sdk_resolved_working_directory()
    -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        let config = Config {
            working_directory: Some(workspace.path().to_path_buf()),
            ..Default::default()
        };

        let working_directory = resolve_working_directory(None, &config);
        let mut registry = SkillRegistry::new(&working_directory);
        registry.register_builtins();
        let registry = Arc::new(RwLock::new(registry));

        let tools = default_tools(working_directory.clone(), registry);
        let bash = tools
            .iter()
            .find(|tool| tool.name() == "Bash")
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Bash tool should be registered",
                )
            })?;

        let result = bash
            .execute(&create_tool_call(
                "sdk-pwd",
                "Bash",
                json!({ "command": "pwd" }),
            ))
            .await?;

        assert!(result.success);
        assert!(
            result
                .output
                .as_deref()
                .unwrap_or_default()
                .contains(&working_directory.to_string_lossy().to_string())
        );
        Ok(())
    }
}

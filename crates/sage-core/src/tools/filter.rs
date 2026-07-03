use crate::error::{SageError, SageResult};
use crate::settings::types::ToolSettings;
use crate::tools::base::Tool;
use std::collections::HashSet;
use std::sync::Arc;

pub fn filter_tools_by_settings(
    tools: Vec<Arc<dyn Tool>>,
    settings: &ToolSettings,
) -> SageResult<Vec<Arc<dyn Tool>>> {
    validate_tool_filter_conflicts(settings)?;
    Ok(tools
        .into_iter()
        .filter(|tool| settings.is_enabled(tool.name()))
        .collect())
}

fn validate_tool_filter_conflicts(settings: &ToolSettings) -> SageResult<()> {
    let enabled = normalized_tool_set(&settings.enabled);
    let disabled = normalized_tool_set(&settings.disabled);
    if let Some(tool) = enabled.intersection(&disabled).next() {
        return Err(SageError::config(format!(
            "Tool '{}' appears in both enabled and disabled settings",
            tool
        )));
    }
    Ok(())
}

fn normalized_tool_set(tools: &[String]) -> HashSet<String> {
    tools
        .iter()
        .map(|tool| tool.trim().to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::{Tool, ToolError};
    use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
    use async_trait::async_trait;

    struct NamedTool(&'static str);

    #[async_trait]
    impl Tool for NamedTool {
        fn name(&self) -> &str {
            self.0
        }

        fn description(&self) -> &str {
            "test tool"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.0, "test tool", Vec::new())
        }

        async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success(&call.id, self.0, "ok"))
        }
    }

    #[test]
    fn filters_enabled_and_disabled_tools_case_insensitively() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(NamedTool("Read")),
            Arc::new(NamedTool("Write")),
            Arc::new(NamedTool("Bash")),
        ];
        let settings = ToolSettings {
            enabled: vec!["read".to_string(), "write".to_string()],
            disabled: vec!["WRITE".to_string()],
            ..Default::default()
        };

        let result = filter_tools_by_settings(tools, &settings);

        assert!(result.is_err());
    }

    #[test]
    fn returns_only_enabled_tools_when_enabled_list_is_present() -> SageResult<()> {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(NamedTool("Read")),
            Arc::new(NamedTool("Write")),
            Arc::new(NamedTool("Bash")),
        ];
        let settings = ToolSettings {
            enabled: vec!["read".to_string(), "write".to_string()],
            disabled: Vec::new(),
            ..Default::default()
        };

        let filtered = filter_tools_by_settings(tools, &settings)?;
        let names = filtered.iter().map(|tool| tool.name()).collect::<Vec<_>>();

        assert_eq!(names, vec!["Read", "Write"]);
        Ok(())
    }

    #[test]
    fn disabled_tools_are_removed_when_enabled_list_is_empty() -> SageResult<()> {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(NamedTool("Read")),
            Arc::new(NamedTool("Write")),
            Arc::new(NamedTool("Bash")),
        ];
        let settings = ToolSettings {
            disabled: vec!["bash".to_string()],
            ..Default::default()
        };

        let filtered = filter_tools_by_settings(tools, &settings)?;
        let names = filtered.iter().map(|tool| tool.name()).collect::<Vec<_>>();

        assert_eq!(names, vec!["Read", "Write"]);
        Ok(())
    }
}

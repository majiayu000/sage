//! Data-driven eval task definitions.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvalSuite {
    pub name: String,
    pub tasks: Vec<EvalTask>,
}

impl EvalSuite {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read eval suite {}", path.display()))?;
        let suite: Self = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse eval suite {}", path.display()))?;
        suite.validate()?;
        Ok(suite)
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("eval suite name must not be empty");
        }
        if self.tasks.is_empty() {
            bail!("eval suite must contain at least one task");
        }
        for task in &self.tasks {
            task.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvalTask {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub required_tool_categories: Vec<String>,
    #[serde(default)]
    pub expected_tool_names: Vec<String>,
    #[serde(default)]
    pub workspace_files: Vec<WorkspaceFile>,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub offline: Option<OfflineTrace>,
}

impl EvalTask {
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("eval task id must not be empty");
        }
        if self.id.contains('/') || self.id.contains('\\') || self.id.contains("..") {
            bail!("eval task id must be a safe path segment: {}", self.id);
        }
        if self.prompt.trim().is_empty() {
            bail!("eval task '{}' prompt must not be empty", self.id);
        }
        for file in &self.workspace_files {
            validate_relative_path(&file.path)
                .with_context(|| format!("invalid workspace file in task '{}'", self.id))?;
        }
        for assertion in &self.assertions {
            if let Assertion::FileContains { path, .. } = assertion {
                validate_relative_path(path)
                    .with_context(|| format!("invalid file assertion in task '{}'", self.id))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceFile {
    pub path: PathBuf,
    pub content: String,
}

impl WorkspaceFile {
    pub async fn write_to(&self, workspace: &Path) -> Result<()> {
        validate_relative_path(&self.path)?;
        let path = workspace.join(&self.path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, &self.content).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Assertion {
    OutputContains { value: String },
    FileContains { path: PathBuf, value: String },
}

impl Assertion {
    pub async fn evaluate(&self, final_output: &str, workspace: &Path) -> Result<bool> {
        match self {
            Self::OutputContains { value } => Ok(final_output.contains(value)),
            Self::FileContains { path, value } => {
                validate_relative_path(path)?;
                let content = tokio::fs::read_to_string(workspace.join(path)).await?;
                Ok(content.contains(value))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfflineTrace {
    pub final_output: String,
    #[serde(default)]
    pub tool_intents: Vec<ToolIntentSpec>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolIntentSpec {
    pub tool_category: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallSpec {
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub output: Option<String>,
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("path must not be empty");
    }
    if path.is_absolute() {
        bail!("path must be relative: {}", path.display());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        bail!("path must stay inside eval workspace: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        let task = EvalTask {
            id: "bad".to_string(),
            prompt: "test".to_string(),
            required_tool_categories: Vec::new(),
            expected_tool_names: Vec::new(),
            workspace_files: vec![WorkspaceFile {
                path: PathBuf::from("../outside.txt"),
                content: String::new(),
            }],
            assertions: Vec::new(),
            offline: None,
        };

        assert!(task.validate().is_err());
    }
}

//! Skill context types

use super::trigger::TaskType;
use std::path::PathBuf;

/// Context for skill matching
#[derive(Debug, Clone, Default)]
pub struct SkillContext {
    /// User's message/prompt
    pub user_message: String,

    /// Current working directory
    pub working_dir: PathBuf,

    /// Active files being worked on
    pub active_files: Vec<PathBuf>,

    /// Recently used tools
    pub recent_tools: Vec<String>,

    /// Explicitly requested skill
    pub explicit_skill: Option<String>,

    /// Detected task type
    pub detected_task_type: Option<TaskType>,

    /// Additional file context
    pub file_context: Option<String>,
}

impl SkillContext {
    /// Create a new context
    pub fn new(user_message: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            ..Default::default()
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = dir.into();
        self
    }

    /// Add active file
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.active_files.push(file.into());
        self
    }

    /// Add recent tool
    pub fn with_recent_tool(mut self, tool: impl Into<String>) -> Self {
        self.recent_tools.push(tool.into());
        self
    }

    /// Set explicit skill request
    pub fn with_explicit_skill(mut self, skill: impl Into<String>) -> Self {
        self.explicit_skill = Some(skill.into());
        self
    }

    /// Set detected task type
    pub fn with_task_type(mut self, task_type: TaskType) -> Self {
        self.detected_task_type = Some(task_type);
        self
    }
}

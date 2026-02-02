//! Embedded prompts fallback
//!
//! Provides compile-time embedded prompts as fallback when file-based prompts
//! are not available.

use super::file_loader::{PromptFile, PromptMetadata};
use std::collections::HashMap;

/// Embedded prompts registry
pub struct EmbeddedPrompts {
    prompts: HashMap<String, PromptFile>,
}

impl EmbeddedPrompts {
    /// Create embedded prompts registry with all built-in prompts
    pub fn new() -> Self {
        let mut prompts = HashMap::new();

        // System prompts
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "identity",
            include_str!("../../../prompts/system-prompt/identity.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "tone-and-style",
            include_str!("../../../prompts/system-prompt/tone-and-style.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "professional-objectivity",
            include_str!("../../../prompts/system-prompt/professional-objectivity.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "doing-tasks",
            include_str!("../../../prompts/system-prompt/doing-tasks.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "asking-questions",
            include_str!("../../../prompts/system-prompt/asking-questions.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "tool-usage-policy",
            include_str!("../../../prompts/system-prompt/tool-usage-policy.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "code-references",
            include_str!("../../../prompts/system-prompt/code-references.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "environment-info",
            include_str!("../../../prompts/system-prompt/environment-info.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "task-management",
            include_str!("../../../prompts/system-prompt/task-management.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "hooks",
            include_str!("../../../prompts/system-prompt/hooks.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "help-and-feedback",
            include_str!("../../../prompts/system-prompt/help-and-feedback.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "documentation-lookup",
            include_str!("../../../prompts/system-prompt/documentation-lookup.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "planning-without-timelines",
            include_str!("../../../prompts/system-prompt/planning-without-timelines.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "system-reminders-info",
            include_str!("../../../prompts/system-prompt/system-reminders-info.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "git-status-section",
            include_str!("../../../prompts/system-prompt/git-status-section.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-prompt",
            "no-time-estimates",
            include_str!("../../../prompts/system-prompt/no-time-estimates.md"),
        );

        // Agent prompts
        Self::add_embedded(
            &mut prompts,
            "agent-prompt",
            "explore",
            include_str!("../../../prompts/agent-prompt/explore.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "agent-prompt",
            "plan",
            include_str!("../../../prompts/agent-prompt/plan.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "agent-prompt",
            "code-review",
            include_str!("../../../prompts/agent-prompt/code-review.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "agent-prompt",
            "general-purpose",
            include_str!("../../../prompts/agent-prompt/general-purpose.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "agent-prompt",
            "claude-guide",
            include_str!("../../../prompts/agent-prompt/claude-guide.md"),
        );

        // Tool descriptions
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "bash",
            include_str!("../../../prompts/tool-description/bash.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "read",
            include_str!("../../../prompts/tool-description/read.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "edit",
            include_str!("../../../prompts/tool-description/edit.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "write",
            include_str!("../../../prompts/tool-description/write.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "glob",
            include_str!("../../../prompts/tool-description/glob.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "grep",
            include_str!("../../../prompts/tool-description/grep.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "task",
            include_str!("../../../prompts/tool-description/task.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "todo-write",
            include_str!("../../../prompts/tool-description/todo-write.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "enter-plan-mode",
            include_str!("../../../prompts/tool-description/enter-plan-mode.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "exit-plan-mode",
            include_str!("../../../prompts/tool-description/exit-plan-mode.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "ask-user-question",
            include_str!("../../../prompts/tool-description/ask-user-question.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "web-fetch",
            include_str!("../../../prompts/tool-description/web-fetch.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "tool-description",
            "web-search",
            include_str!("../../../prompts/tool-description/web-search.md"),
        );

        // Git prompts
        Self::add_embedded(
            &mut prompts,
            "git",
            "safety-protocol",
            include_str!("../../../prompts/git/safety-protocol.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "git",
            "pr-creation",
            include_str!("../../../prompts/git/pr-creation.md"),
        );

        // Security prompts
        Self::add_embedded(
            &mut prompts,
            "security",
            "main-policy",
            include_str!("../../../prompts/security/main-policy.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "security",
            "code-security",
            include_str!("../../../prompts/security/code-security.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "security",
            "bash-security",
            include_str!("../../../prompts/security/bash-security.md"),
        );

        // System reminders
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "plan-mode-active",
            include_str!("../../../prompts/system-reminder/plan-mode-active.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "plan-mode-subagent",
            include_str!("../../../prompts/system-reminder/plan-mode-subagent.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "plan-mode-re-entry",
            include_str!("../../../prompts/system-reminder/plan-mode-re-entry.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "task-completion",
            include_str!("../../../prompts/system-reminder/task-completion.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "todo-list-status",
            include_str!("../../../prompts/system-reminder/todo-list-status.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "file-operation-warning",
            include_str!("../../../prompts/system-reminder/file-operation-warning.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "team-coordination",
            include_str!("../../../prompts/system-reminder/team-coordination.md"),
        );
        Self::add_embedded(
            &mut prompts,
            "system-reminder",
            "delegate-mode",
            include_str!("../../../prompts/system-reminder/delegate-mode.md"),
        );

        Self { prompts }
    }

    /// Add an embedded prompt
    fn add_embedded(
        prompts: &mut HashMap<String, PromptFile>,
        category: &str,
        name: &str,
        content: &str,
    ) {
        let key = format!("{}/{}", category, name);
        if let Ok(prompt) = PromptFile::parse(content) {
            prompts.insert(key, prompt);
        } else {
            // If parsing fails, create a minimal prompt
            prompts.insert(
                key,
                PromptFile {
                    metadata: PromptMetadata {
                        name: name.to_string(),
                        category: category.to_string(),
                        ..Default::default()
                    },
                    content: content.to_string(),
                    source_path: None,
                },
            );
        }
    }

    /// Get an embedded prompt
    pub fn get(&self, category: &str, name: &str) -> Option<&PromptFile> {
        let key = format!("{}/{}", category, name);
        self.prompts.get(&key)
    }

    /// Get an embedded prompt by key
    pub fn get_by_key(&self, key: &str) -> Option<&PromptFile> {
        self.prompts.get(key)
    }

    /// Get all prompts in a category
    pub fn get_category(&self, category: &str) -> Vec<&PromptFile> {
        let prefix = format!("{}/", category);
        self.prompts
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v)
            .collect()
    }

    /// List all embedded prompt keys
    pub fn list_keys(&self) -> Vec<&str> {
        self.prompts.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an embedded prompt exists
    pub fn contains(&self, category: &str, name: &str) -> bool {
        let key = format!("{}/{}", category, name);
        self.prompts.contains_key(&key)
    }

    /// Get the number of embedded prompts
    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }
}

impl Default for EmbeddedPrompts {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_prompts_loaded() {
        let embedded = EmbeddedPrompts::new();
        assert!(!embedded.is_empty());
    }

    #[test]
    fn test_get_system_prompt() {
        let embedded = EmbeddedPrompts::new();
        let prompt = embedded.get("system-prompt", "identity");
        assert!(prompt.is_some());
        assert!(prompt.unwrap().content.contains("${AGENT_NAME}"));
    }

    #[test]
    fn test_get_agent_prompt() {
        let embedded = EmbeddedPrompts::new();
        let prompt = embedded.get("agent-prompt", "explore");
        assert!(prompt.is_some());
        assert!(prompt.unwrap().content.contains("READ-ONLY"));
    }

    #[test]
    fn test_get_tool_description() {
        let embedded = EmbeddedPrompts::new();
        let prompt = embedded.get("tool-description", "bash");
        assert!(prompt.is_some());
    }

    #[test]
    fn test_get_category() {
        let embedded = EmbeddedPrompts::new();
        let system_prompts = embedded.get_category("system-prompt");
        assert!(!system_prompts.is_empty());
    }

    #[test]
    fn test_list_keys() {
        let embedded = EmbeddedPrompts::new();
        let keys = embedded.list_keys();
        assert!(keys.contains(&"system-prompt/identity"));
        assert!(keys.contains(&"agent-prompt/explore"));
        assert!(keys.contains(&"tool-description/bash"));
    }
}

//! Prompt variable system
//!
//! Provides dynamic variable substitution for prompts,
//! following Claude Code's ${VAR} template pattern.

use std::collections::{HashMap, HashSet};

/// Variables available for prompt templates
#[derive(Debug, Clone, Default)]
pub struct PromptVariables {
    /// Tool names
    pub bash_tool_name: String,
    pub read_tool_name: String,
    pub edit_tool_name: String,
    pub write_tool_name: String,
    pub glob_tool_name: String,
    pub grep_tool_name: String,
    pub task_tool_name: String,
    pub todo_tool_name: String,
    pub ask_user_question_tool_name: String,
    pub web_fetch_tool_name: String,
    pub enter_plan_mode_tool_name: String,
    pub exit_plan_mode_tool_name: String,

    /// Available tools set (for conditional rendering)
    pub available_tools: HashSet<String>,

    /// Agent types
    pub explore_agent_type: String,
    pub plan_agent_type: String,
    pub code_review_agent_type: String,
    pub guide_agent_type: String,

    /// Feedback and docs
    pub feedback_url: String,
    pub docs_url: String,

    /// Version tracking
    pub prompt_version: String,

    /// Environment info
    pub working_dir: String,
    pub platform: String,
    pub os_version: String,
    pub current_date: String,
    pub is_git_repo: bool,
    pub git_branch: String,
    pub main_branch: String,

    /// Identity info
    pub agent_name: String,
    pub agent_version: String,
    pub model_name: String,

    /// Task info
    pub task_description: String,

    /// Plan mode
    pub in_plan_mode: bool,
    pub plan_file_path: String,
    pub plan_exists: bool,

    /// Custom variables
    pub custom: HashMap<String, String>,
}

impl PromptVariables {
    /// Create with default tool names
    pub fn new() -> Self {
        Self {
            bash_tool_name: "Bash".to_string(),
            read_tool_name: "Read".to_string(),
            edit_tool_name: "Edit".to_string(),
            write_tool_name: "Write".to_string(),
            glob_tool_name: "Glob".to_string(),
            grep_tool_name: "Grep".to_string(),
            task_tool_name: "Task".to_string(),
            todo_tool_name: "TodoWrite".to_string(),
            ask_user_question_tool_name: "AskUserQuestion".to_string(),
            web_fetch_tool_name: "WebFetch".to_string(),
            enter_plan_mode_tool_name: "EnterPlanMode".to_string(),
            exit_plan_mode_tool_name: "ExitPlanMode".to_string(),

            available_tools: HashSet::new(),

            explore_agent_type: "Explore".to_string(),
            plan_agent_type: "Plan".to_string(),
            code_review_agent_type: "code-review".to_string(),
            guide_agent_type: "guide".to_string(),

            feedback_url: "https://github.com/anthropics/sage/issues".to_string(),
            docs_url: "https://docs.sage-agent.dev".to_string(),

            prompt_version: "1.0.0".to_string(),

            working_dir: ".".to_string(),
            platform: std::env::consts::OS.to_string(),
            os_version: "".to_string(),
            current_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            is_git_repo: false,
            git_branch: "".to_string(),
            main_branch: "main".to_string(),

            agent_name: "Sage Agent".to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            model_name: "".to_string(),

            task_description: "".to_string(),

            in_plan_mode: false,
            plan_file_path: "".to_string(),
            plan_exists: false,

            custom: HashMap::new(),
        }
    }

    /// Check if a tool is available
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.available_tools.contains(tool_name)
    }

    /// Add a tool to available set
    pub fn add_tool(&mut self, tool_name: impl Into<String>) {
        self.available_tools.insert(tool_name.into());
    }

    /// Set custom variable
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom.insert(key.into(), value.into());
    }

    /// Get variable value by name (returns reference to avoid cloning)
    pub fn get(&self, name: &str) -> Option<&str> {
        match name {
            "BASH_TOOL_NAME" => Some(&self.bash_tool_name),
            "READ_TOOL_NAME" => Some(&self.read_tool_name),
            "EDIT_TOOL_NAME" => Some(&self.edit_tool_name),
            "WRITE_TOOL_NAME" => Some(&self.write_tool_name),
            "GLOB_TOOL_NAME" => Some(&self.glob_tool_name),
            "GREP_TOOL_NAME" => Some(&self.grep_tool_name),
            "TASK_TOOL_NAME" => Some(&self.task_tool_name),
            "TODO_TOOL_NAME" => Some(&self.todo_tool_name),
            "ASK_USER_QUESTION_TOOL_NAME" => Some(&self.ask_user_question_tool_name),
            "WEB_FETCH_TOOL_NAME" => Some(&self.web_fetch_tool_name),
            "ENTER_PLAN_MODE_TOOL_NAME" => Some(&self.enter_plan_mode_tool_name),
            "EXIT_PLAN_MODE_TOOL_NAME" => Some(&self.exit_plan_mode_tool_name),

            "EXPLORE_AGENT_TYPE" => Some(&self.explore_agent_type),
            "PLAN_AGENT_TYPE" => Some(&self.plan_agent_type),
            "CODE_REVIEW_AGENT_TYPE" => Some(&self.code_review_agent_type),
            "GUIDE_AGENT_TYPE" => Some(&self.guide_agent_type),

            "FEEDBACK_URL" => Some(&self.feedback_url),
            "DOCS_URL" => Some(&self.docs_url),

            "PROMPT_VERSION" => Some(&self.prompt_version),

            "WORKING_DIR" => Some(&self.working_dir),
            "PLATFORM" => Some(&self.platform),
            "OS_VERSION" => Some(&self.os_version),
            "CURRENT_DATE" => Some(&self.current_date),
            "GIT_BRANCH" => Some(&self.git_branch),
            "MAIN_BRANCH" => Some(&self.main_branch),

            "AGENT_NAME" => Some(&self.agent_name),
            "AGENT_VERSION" => Some(&self.agent_version),
            "MODEL_NAME" => Some(&self.model_name),

            "TASK_DESCRIPTION" => Some(&self.task_description),
            "PLAN_FILE_PATH" => Some(&self.plan_file_path),

            _ => self.custom.get(name).map(|s| s.as_str()),
        }
    }
}

/// Template renderer for prompt strings
pub struct TemplateRenderer;

impl TemplateRenderer {
    /// Render a template string with variables
    ///
    /// Supports:
    /// - Simple substitution: ${VAR_NAME}
    /// - Conditional sections: ${CONDITION?`content if true`:`content if false`}
    pub fn render(template: &str, vars: &PromptVariables) -> String {
        let mut result = template.to_string();

        // First pass: handle conditional sections
        result = Self::render_conditionals(&result, vars);

        // Second pass: simple variable substitution
        result = Self::render_variables(&result, vars);

        result
    }

    /// Render conditional sections
    fn render_conditionals(template: &str, vars: &PromptVariables) -> String {
        let mut result = template.to_string();

        // Pattern: ${HAS_TOOL_XXX?`...`:`...`}
        // Simple implementation - can be enhanced with regex

        // Handle HAS_TOOL conditions
        for tool in &[
            "Bash",
            "Read",
            "Edit",
            "Write",
            "Glob",
            "Grep",
            "Task",
            "TodoWrite",
            "AskUserQuestion",
            "WebFetch",
            "EnterPlanMode",
            "ExitPlanMode",
        ] {
            let has_tool = vars.has_tool(tool);
            let pattern = format!("${{HAS_TOOL_{}?`", tool.to_uppercase());

            if let Some(start) = result.find(&pattern) {
                if let Some(content) = Self::extract_conditional(&result[start..], has_tool) {
                    let end = Self::find_conditional_end(&result[start..]);
                    result = format!("{}{}{}", &result[..start], content, &result[start + end..]);
                }
            }
        }

        // Handle IN_PLAN_MODE condition
        let pattern = "${IN_PLAN_MODE?`";
        while let Some(start) = result.find(pattern) {
            if let Some(content) = Self::extract_conditional(&result[start..], vars.in_plan_mode) {
                let end = Self::find_conditional_end(&result[start..]);
                result = format!("{}{}{}", &result[..start], content, &result[start + end..]);
            } else {
                break;
            }
        }

        // Handle IS_GIT_REPO condition
        let pattern = "${IS_GIT_REPO?`";
        while let Some(start) = result.find(pattern) {
            if let Some(content) = Self::extract_conditional(&result[start..], vars.is_git_repo) {
                let end = Self::find_conditional_end(&result[start..]);
                result = format!("{}{}{}", &result[..start], content, &result[start + end..]);
            } else {
                break;
            }
        }

        // Handle PLAN_EXISTS condition
        let pattern = "${PLAN_EXISTS?`";
        while let Some(start) = result.find(pattern) {
            if let Some(content) = Self::extract_conditional(&result[start..], vars.plan_exists) {
                let end = Self::find_conditional_end(&result[start..]);
                result = format!("{}{}{}", &result[..start], content, &result[start + end..]);
            } else {
                break;
            }
        }

        result
    }

    /// Extract conditional content based on condition value
    fn extract_conditional(s: &str, condition: bool) -> Option<String> {
        // Find the pattern: ${COND?`true_content`:`false_content`}
        let true_start = s.find('`')? + 1;
        let mut depth = 1;
        let mut true_end = true_start;
        let chars: Vec<char> = s.chars().collect();

        // Find end of true content (matching backtick)
        while true_end < chars.len() && depth > 0 {
            if chars[true_end] == '`' {
                if true_end > 0 && chars[true_end - 1] != '\\' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            } else if chars[true_end] == '$'
                && true_end + 1 < chars.len()
                && chars[true_end + 1] == '{'
            {
                // Skip nested conditionals
                if let Some(nested_end) = s[true_end..].find('}') {
                    true_end += nested_end;
                }
            }
            true_end += 1;
        }

        let true_content: String = chars[true_start..true_end].iter().collect();

        // Find false content after `:`
        let false_marker = true_end + 2; // Skip `:`
        if false_marker >= chars.len() || chars[false_marker] != '`' {
            return if condition {
                Some(true_content)
            } else {
                Some(String::new())
            };
        }

        let false_start = false_marker + 1;
        let mut false_end = false_start;
        depth = 1;

        while false_end < chars.len() && depth > 0 {
            if chars[false_end] == '`' {
                if false_end > 0 && chars[false_end - 1] != '\\' {
                    depth -= 1;
                }
            }
            if depth > 0 {
                false_end += 1;
            }
        }

        let false_content: String = chars[false_start..false_end].iter().collect();

        if condition {
            Some(true_content)
        } else {
            Some(false_content)
        }
    }

    /// Find the end of a conditional expression
    fn find_conditional_end(s: &str) -> usize {
        let mut depth = 0;
        let mut in_backtick = false;

        for (i, c) in s.chars().enumerate() {
            match c {
                '{' if !in_backtick => depth += 1,
                '}' if !in_backtick => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                '`' => in_backtick = !in_backtick,
                _ => {}
            }
        }

        s.len()
    }

    /// Simple variable substitution
    fn render_variables(template: &str, vars: &PromptVariables) -> String {
        let mut result = template.to_string();

        // Replace all ${VAR_NAME} patterns
        let var_names = [
            "BASH_TOOL_NAME",
            "READ_TOOL_NAME",
            "EDIT_TOOL_NAME",
            "WRITE_TOOL_NAME",
            "GLOB_TOOL_NAME",
            "GREP_TOOL_NAME",
            "TASK_TOOL_NAME",
            "TODO_TOOL_NAME",
            "ASK_USER_QUESTION_TOOL_NAME",
            "WEB_FETCH_TOOL_NAME",
            "ENTER_PLAN_MODE_TOOL_NAME",
            "EXIT_PLAN_MODE_TOOL_NAME",
            "EXPLORE_AGENT_TYPE",
            "PLAN_AGENT_TYPE",
            "CODE_REVIEW_AGENT_TYPE",
            "GUIDE_AGENT_TYPE",
            "FEEDBACK_URL",
            "DOCS_URL",
            "PROMPT_VERSION",
            "WORKING_DIR",
            "PLATFORM",
            "OS_VERSION",
            "CURRENT_DATE",
            "GIT_BRANCH",
            "MAIN_BRANCH",
            "AGENT_NAME",
            "AGENT_VERSION",
            "MODEL_NAME",
            "TASK_DESCRIPTION",
            "PLAN_FILE_PATH",
        ];

        for var_name in var_names {
            let pattern = format!("${{{}}}", var_name);
            if let Some(value) = vars.get(var_name) {
                result = result.replace(&pattern, &value);
            }
        }

        // Replace custom variables
        for (key, value) in &vars.custom {
            let pattern = format!("${{{}}}", key);
            result = result.replace(&pattern, value);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable_substitution() {
        let vars = PromptVariables::new();
        let template = "Use the ${BASH_TOOL_NAME} tool to run commands.";
        let result = TemplateRenderer::render(template, &vars);
        assert_eq!(result, "Use the Bash tool to run commands.");
    }

    #[test]
    fn test_multiple_variables() {
        let vars = PromptVariables::new();
        let template = "Use ${READ_TOOL_NAME} to read and ${EDIT_TOOL_NAME} to edit.";
        let result = TemplateRenderer::render(template, &vars);
        assert_eq!(result, "Use Read to read and Edit to edit.");
    }

    #[test]
    fn test_custom_variable() {
        let mut vars = PromptVariables::new();
        vars.set("CUSTOM_VAR", "custom_value");
        let template = "Custom: ${CUSTOM_VAR}";
        let result = TemplateRenderer::render(template, &vars);
        assert_eq!(result, "Custom: custom_value");
    }

    #[test]
    fn test_has_tool() {
        let mut vars = PromptVariables::new();
        vars.add_tool("Bash");
        assert!(vars.has_tool("Bash"));
        assert!(!vars.has_tool("Unknown"));
    }
}

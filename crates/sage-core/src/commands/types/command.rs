//! Slash command definition types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A slash command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    /// Command name (without leading slash)
    pub name: String,
    /// Command description
    pub description: Option<String>,
    /// Source file path
    pub source_path: PathBuf,
    /// The prompt template (content of .md file)
    pub prompt_template: String,
    /// Whether this is a built-in command
    pub is_builtin: bool,
    /// Command arguments definition
    pub arguments: Vec<CommandArgument>,
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Allowed tools (None = all tools allowed)
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Model override (None = use default model)
    #[serde(default)]
    pub model_override: Option<String>,
}

impl SlashCommand {
    /// Create a new slash command
    pub fn new(name: impl Into<String>, prompt_template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            source_path: PathBuf::new(),
            prompt_template: prompt_template.into(),
            is_builtin: false,
            arguments: Vec::new(),
            required_permissions: Vec::new(),
            allowed_tools: None,
            model_override: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set source path
    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = path.into();
        self
    }

    /// Mark as builtin
    pub fn builtin(mut self) -> Self {
        self.is_builtin = true;
        self
    }

    /// Add an argument
    pub fn with_argument(mut self, arg: CommandArgument) -> Self {
        self.arguments.push(arg);
        self
    }

    /// Add required permission
    pub fn with_permission(mut self, permission: impl Into<String>) -> Self {
        self.required_permissions.push(permission.into());
        self
    }

    /// Set allowed tools
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools);
        self
    }

    /// Set model override
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_override = Some(model.into());
        self
    }

    /// Expand the prompt template with arguments
    pub fn expand(&self, args: &[String]) -> String {
        let mut result = self.prompt_template.clone();

        // Replace $ARGUMENTS_JSON first
        if let Ok(json) = serde_json::to_string(args) {
            result = result.replace("$ARGUMENTS_JSON", &json);
        }

        // Replace $ARGUMENTS with all arguments joined
        let all_args = args.join(" ");
        result = result.replace("$ARGUMENTS", &all_args);

        // Replace $ARG1, $ARG2, etc.
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("$ARG{}", i + 1), arg);
        }

        result
    }

    /// Check if this command requires arguments
    pub fn requires_arguments(&self) -> bool {
        self.prompt_template.contains("$ARGUMENTS") || self.prompt_template.contains("$ARG1")
    }

    /// Get minimum required argument count
    pub fn min_args(&self) -> usize {
        let mut max_arg = 0;
        for i in 1..=10 {
            if self.prompt_template.contains(&format!("$ARG{}", i)) {
                max_arg = i;
            }
        }
        self.arguments
            .iter()
            .filter(|a| a.required)
            .count()
            .max(max_arg)
    }
}

/// Command argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArgument {
    /// Argument name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Whether this argument is required
    pub required: bool,
    /// Default value
    pub default: Option<String>,
}

impl CommandArgument {
    /// Create a new required argument
    pub fn required(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            required: true,
            default: None,
        }
    }

    /// Create a new optional argument
    pub fn optional(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            required: false,
            default: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self.required = false;
        self
    }
}

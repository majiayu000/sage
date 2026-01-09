//! Slash command type definitions
//!
//! This module defines types for the slash command system,
//! allowing users to define custom commands in `.sage/commands/`.

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

    /// Expand the prompt template with arguments
    pub fn expand(&self, args: &[String]) -> String {
        let mut result = self.prompt_template.clone();

        // Replace $ARGUMENTS_JSON first (before $ARGUMENTS to avoid partial replacement)
        if let Ok(json) = serde_json::to_string(args) {
            result = result.replace("$ARGUMENTS_JSON", &json);
        }

        // Replace $ARGUMENTS with all arguments joined
        let all_args = args.join(" ");
        result = result.replace("$ARGUMENTS", &all_args);

        // Replace $ARG1, $ARG2, etc. with individual arguments
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

/// Result of parsing a slash command invocation
#[derive(Debug, Clone)]
pub struct CommandInvocation {
    /// The command name
    pub command_name: String,

    /// Arguments passed to the command
    pub arguments: Vec<String>,

    /// Raw input string
    pub raw_input: String,
}

impl CommandInvocation {
    /// Parse a slash command from input string
    ///
    /// Format: /command-name arg1 arg2 ...
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();

        if !input.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = input[1..].splitn(2, char::is_whitespace).collect();

        let command_name = parts.first()?.to_string();
        if command_name.is_empty() {
            return None;
        }

        let arguments = if parts.len() > 1 {
            shell_words::split(parts[1])
                .unwrap_or_else(|_| parts[1].split_whitespace().map(String::from).collect())
        } else {
            Vec::new()
        };

        Some(Self {
            command_name,
            arguments,
            raw_input: input.to_string(),
        })
    }

    /// Check if this looks like a slash command
    pub fn is_slash_command(input: &str) -> bool {
        let input = input.trim();
        input.starts_with('/')
            && input.len() > 1
            && input.chars().nth(1).map_or(false, |c| c.is_alphabetic())
    }
}

/// Interactive command type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveCommand {
    /// Resume a session (optionally with a specific session ID)
    Resume {
        session_id: Option<String>,
        show_all: bool,
    },
    /// Set custom title for current session (Claude Code style)
    Title { title: String },
    /// Run login/credential setup wizard
    Login,
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Expanded prompt to send to LLM
    pub expanded_prompt: String,

    /// Whether to show the expansion to user
    pub show_expansion: bool,

    /// Additional context messages to prepend
    pub context_messages: Vec<String>,

    /// Status message to display
    pub status_message: Option<String>,

    /// Whether this is a local command (output directly, don't send to LLM)
    pub is_local: bool,

    /// Local output to display (for local commands)
    pub local_output: Option<String>,

    /// Interactive command that needs CLI handling
    pub interactive: Option<InteractiveCommand>,
}

impl CommandResult {
    /// Create a simple result with expanded prompt
    pub fn prompt(expanded_prompt: impl Into<String>) -> Self {
        Self {
            expanded_prompt: expanded_prompt.into(),
            show_expansion: false,
            context_messages: Vec::new(),
            status_message: None,
            is_local: false,
            local_output: None,
            interactive: None,
        }
    }

    /// Create a local command result (displayed directly, not sent to LLM)
    pub fn local(output: impl Into<String>) -> Self {
        Self {
            expanded_prompt: String::new(),
            show_expansion: false,
            context_messages: Vec::new(),
            status_message: None,
            is_local: true,
            local_output: Some(output.into()),
            interactive: None,
        }
    }

    /// Create an interactive command result
    pub fn interactive(cmd: InteractiveCommand) -> Self {
        Self {
            expanded_prompt: String::new(),
            show_expansion: false,
            context_messages: Vec::new(),
            status_message: None,
            is_local: true,
            local_output: None,
            interactive: Some(cmd),
        }
    }

    /// Show the expansion to user
    pub fn show(mut self) -> Self {
        self.show_expansion = true;
        self
    }

    /// Add context message
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context_messages.push(context.into());
        self
    }

    /// Set status message
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status_message = Some(status.into());
        self
    }

    /// Check if this is an interactive command
    pub fn is_interactive(&self) -> bool {
        self.interactive.is_some()
    }
}

/// Command source location
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSource {
    /// Built-in command
    Builtin,
    /// Project-level command (.sage/commands/)
    Project,
    /// User-level command (~/.config/sage/commands/)
    User,
}

impl std::fmt::Display for CommandSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin => write!(f, "builtin"),
            Self::Project => write!(f, "project"),
            Self::User => write!(f, "user"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = SlashCommand::new("test", "Run tests").with_description("Run all tests");

        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, Some("Run all tests".to_string()));
    }

    #[test]
    fn test_command_expand_arguments() {
        let cmd = SlashCommand::new("greet", "Hello $ARGUMENTS!");
        let expanded = cmd.expand(&["World".to_string()]);
        assert_eq!(expanded, "Hello World!");
    }

    #[test]
    fn test_command_expand_numbered_args() {
        let cmd = SlashCommand::new("swap", "Swap $ARG1 with $ARG2");
        let expanded = cmd.expand(&["foo".to_string(), "bar".to_string()]);
        assert_eq!(expanded, "Swap foo with bar");
    }

    #[test]
    fn test_command_expand_json_args() {
        let cmd = SlashCommand::new("list", "Items: $ARGUMENTS_JSON");
        let expanded = cmd.expand(&["a".to_string(), "b".to_string()]);
        // JSON might have spaces, so check for both elements
        assert!(expanded.contains("\"a\""));
        assert!(expanded.contains("\"b\""));
    }

    #[test]
    fn test_command_requires_arguments() {
        let cmd1 = SlashCommand::new("simple", "Just text");
        assert!(!cmd1.requires_arguments());

        let cmd2 = SlashCommand::new("with_args", "Process $ARGUMENTS");
        assert!(cmd2.requires_arguments());
    }

    #[test]
    fn test_command_min_args() {
        let cmd1 = SlashCommand::new("none", "No args");
        assert_eq!(cmd1.min_args(), 0);

        let cmd2 = SlashCommand::new("two", "$ARG1 and $ARG2");
        assert_eq!(cmd2.min_args(), 2);
    }

    #[test]
    fn test_parse_invocation() {
        let inv = CommandInvocation::parse("/test arg1 arg2").unwrap();
        assert_eq!(inv.command_name, "test");
        assert_eq!(inv.arguments, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_parse_invocation_no_args() {
        let inv = CommandInvocation::parse("/help").unwrap();
        assert_eq!(inv.command_name, "help");
        assert!(inv.arguments.is_empty());
    }

    #[test]
    fn test_parse_invocation_quoted_args() {
        let inv = CommandInvocation::parse("/search \"hello world\"").unwrap();
        assert_eq!(inv.command_name, "search");
        assert_eq!(inv.arguments, vec!["hello world"]);
    }

    #[test]
    fn test_parse_invocation_not_slash() {
        assert!(CommandInvocation::parse("test").is_none());
        assert!(CommandInvocation::parse("").is_none());
    }

    #[test]
    fn test_is_slash_command() {
        assert!(CommandInvocation::is_slash_command("/test"));
        assert!(CommandInvocation::is_slash_command("/help arg"));
        assert!(!CommandInvocation::is_slash_command("test"));
        assert!(!CommandInvocation::is_slash_command("/"));
        assert!(!CommandInvocation::is_slash_command("/123"));
    }

    #[test]
    fn test_command_result() {
        let result = CommandResult::prompt("Hello")
            .show()
            .with_context("Context 1")
            .with_status("Running...");

        assert_eq!(result.expanded_prompt, "Hello");
        assert!(result.show_expansion);
        assert_eq!(result.context_messages.len(), 1);
        assert_eq!(result.status_message, Some("Running...".to_string()));
    }

    #[test]
    fn test_command_argument() {
        let arg = CommandArgument::required("file").with_description("The file to process");

        assert_eq!(arg.name, "file");
        assert!(arg.required);
        assert!(arg.default.is_none());

        let opt = CommandArgument::optional("format").with_default("json");

        assert!(!opt.required);
        assert_eq!(opt.default, Some("json".to_string()));
    }
}

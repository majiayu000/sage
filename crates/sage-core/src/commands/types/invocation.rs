//! Command invocation parsing

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

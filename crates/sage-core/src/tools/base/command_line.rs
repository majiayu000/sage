//! Command line parsing helpers for safe execution.

use shell_words::split;

use super::ToolError;

/// Parsed command line with program and args separated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandLine {
    /// Parse a command line string into argv.
    ///
    /// This rejects shell control operators to avoid accidental shell semantics.
    pub fn parse(command: &str) -> Result<Self, ToolError> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command must not be empty".to_string(),
            ));
        }

        if contains_shell_operators(trimmed) {
            return Err(ToolError::InvalidArguments(
                "Shell operators are not allowed. Provide a single command without pipes, redirection, or command chaining."
                    .to_string(),
            ));
        }

        let argv = split(trimmed).map_err(|e| {
            ToolError::InvalidArguments(format!("Failed to parse command line: {}", e))
        })?;

        let (program, args) = argv
            .split_first()
            .map(|(p, rest)| (p.clone(), rest.to_vec()))
            .ok_or_else(|| {
                ToolError::InvalidArguments("Command must include a program".to_string())
            })?;

        Ok(Self { program, args })
    }

    /// Return argv vector including program.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(1 + self.args.len());
        argv.push(self.program.clone());
        argv.extend(self.args.clone());
        argv
    }
}

fn contains_shell_operators(command: &str) -> bool {
    let bytes = command.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'|' | b'&' | b';' | b'<' | b'>' | b'`' => return true,
            b'$' if i + 1 < bytes.len() && bytes[i + 1] == b'(' => return true,
            _ => {}
        }
        i += 1;
    }
    false
}


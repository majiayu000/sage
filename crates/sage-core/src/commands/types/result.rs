//! Command execution result types

/// Interactive command type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveCommand {
    /// Resume a session (optionally with a specific session ID)
    Resume {
        session_id: Option<String>,
        show_all: bool,
    },
    /// Set custom title for current session
    Title { title: String },
    /// Run login/credential setup wizard
    Login,
    /// Clear stored credentials
    Logout,
    /// Switch output display mode
    OutputMode { mode: String },
    /// Switch to a different model
    Model { model: String },
    /// Clear conversation history
    Clear,
    /// Exit the application
    Exit,
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
    /// Tool restrictions (None = all tools allowed)
    pub tool_restrictions: Option<Vec<String>>,
    /// Model override (None = use default model)
    pub model_override: Option<String>,
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
            tool_restrictions: None,
            model_override: None,
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
            tool_restrictions: None,
            model_override: None,
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
            tool_restrictions: None,
            model_override: None,
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

    /// Set tool restrictions
    pub fn with_tool_restrictions(mut self, tools: Vec<String>) -> Self {
        self.tool_restrictions = Some(tools);
        self
    }

    /// Set model override
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_override = Some(model.into());
        self
    }

    /// Check if this is an interactive command
    pub fn is_interactive(&self) -> bool {
        self.interactive.is_some()
    }

    /// Check if this result has tool restrictions
    pub fn has_tool_restrictions(&self) -> bool {
        self.tool_restrictions.is_some()
    }

    /// Check if this result has a model override
    pub fn has_model_override(&self) -> bool {
        self.model_override.is_some()
    }
}

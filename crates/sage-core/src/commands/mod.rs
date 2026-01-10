//! Slash commands system
//!
//! This module provides a slash command system that allows users to define
//! custom commands via markdown files in `.sage/commands/` directories.
//!
//! # Overview
//!
//! Slash commands are triggered by typing `/command-name [args]` in the chat.
//! Commands can be categorized as:
//! - **System**: Core commands like `/help`, `/clear`, `/checkpoint`
//! - **User**: Defined in `.sage/commands/*.md` or `~/.config/sage/commands/*.md`
//! - **MCP**: Commands from MCP (Model Context Protocol) servers
//!
//! # Command Priority
//!
//! When multiple commands have the same name:
//! 1. System commands (highest priority)
//! 2. Project commands
//! 3. User commands
//! 4. MCP commands (lowest priority)
//!
//! # Using the Command Router
//!
//! The `CommandRouter` provides a unified entry point for all command operations:
//!
//! ```rust,ignore
//! use sage_core::commands::CommandRouter;
//!
//! // Create router
//! let router = CommandRouter::new("./project").await?;
//!
//! // Check if input is a command
//! if CommandRouter::is_command("/help") {
//!     // Route and execute
//!     if let Some(result) = router.route("/help").await? {
//!         match result.kind() {
//!             CommandResultKind::Local { output } => println!("{}", output),
//!             CommandResultKind::Prompt { content } => send_to_llm(content),
//!             CommandResultKind::Interactive(cmd) => handle_interactive(cmd),
//!             CommandResultKind::Empty => {}
//!         }
//!     }
//! }
//! ```
//!
//! # Creating Custom Commands
//!
//! Create a markdown file in `.sage/commands/` with the command name:
//!
//! ```markdown
//! ---
//! description: Search the codebase for a pattern
//! ---
//! Search the codebase for "$ARGUMENTS" and show relevant matches.
//! Focus on implementation details and usage patterns.
//! ```
//!
//! ## Template Variables
//!
//! - `$ARGUMENTS` - All arguments as a single string
//! - `$ARG1`, `$ARG2`, etc. - Individual arguments
//! - `$ARGUMENTS_JSON` - Arguments as a JSON array
//!
//! # System Commands
//!
//! | Command | Description |
//! |---------|-------------|
//! | `/help` | Show help information |
//! | `/clear` | Clear conversation history |
//! | `/compact` | Summarize and compact context |
//! | `/checkpoint [name]` | Create a state checkpoint |
//! | `/restore [id]` | Restore to a checkpoint |
//! | `/tasks` | List background tasks |
//! | `/commands` | List all commands |
//! | `/config` | Show/modify configuration |
//! | `/init` | Initialize .sage directory |
//! | `/login` | Configure API credentials |
//! | `/resume` | Resume previous session |

pub mod executor;
pub mod registry;
pub mod router;
pub mod types;

pub use executor::CommandExecutor;
pub use registry::CommandRegistry;
pub use router::{CommandCategory, CommandList, CommandResultKind, CommandRouter, RoutedCommand};
pub use types::{
    CommandArgument, CommandInvocation, CommandResult, CommandSource, InteractiveCommand,
    SlashCommand,
};

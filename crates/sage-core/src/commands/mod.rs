//! Slash commands system
//!
//! This module provides a slash command system that allows users to define
//! custom commands via markdown files in `.sage/commands/` directories.
//!
//! # Overview
//!
//! Slash commands are triggered by typing `/command-name [args]` in the chat.
//! Commands can be:
//! - **Built-in**: Core commands like `/help`, `/clear`, `/checkpoint`
//! - **Project-level**: Defined in `.sage/commands/*.md`
//! - **User-level**: Defined in `~/.config/sage/commands/*.md`
//!
//! # Command Priority
//!
//! When multiple commands have the same name:
//! 1. Built-in commands (highest priority)
//! 2. Project commands
//! 3. User commands (lowest priority)
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
//! # Example Usage
//!
//! ```rust,ignore
//! use sage_core::commands::{CommandRegistry, CommandExecutor};
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! // Create registry and discover commands
//! let mut registry = CommandRegistry::new("./project");
//! registry.register_builtins();
//! registry.discover().await?;
//!
//! // Create executor
//! let executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));
//!
//! // Process user input
//! if let Some(result) = executor.process("/search MyFunction").await? {
//!     println!("Expanded: {}", result.expanded_prompt);
//! }
//! ```
//!
//! # Built-in Commands
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

pub mod executor;
pub mod registry;
pub mod types;

pub use executor::CommandExecutor;
pub use registry::CommandRegistry;
pub use types::{CommandArgument, CommandInvocation, CommandResult, CommandSource, SlashCommand};

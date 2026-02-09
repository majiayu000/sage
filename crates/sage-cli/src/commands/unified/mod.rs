//! Unified command implementation using the Claude Code style execution loop
//!
//! This module implements the new unified execution model where:
//! - There's no distinction between "run" and "interactive" modes
//! - User input blocks inline via InputChannel
//! - The execution loop never exits for user input

mod args;
mod execute;
mod input;
mod mcp;
mod outcome;
mod session;
pub mod slash_commands;
mod stream;
mod utils;

pub use args::{OutputModeArg, UnifiedArgs};
pub use execute::execute;
pub(crate) use input::handle_user_input;
pub(crate) use mcp::build_mcp_registry_from_config;

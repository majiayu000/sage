//! Slash command type definitions
//!
//! This module defines types for the slash command system,
//! allowing users to define custom commands in `.sage/commands/`.

mod command;
mod invocation;
mod result;

pub use command::{CommandArgument, SlashCommand};
pub use invocation::{CommandInvocation, CommandSource};
pub use result::{CommandResult, InteractiveCommand};

#[cfg(test)]
mod tests;

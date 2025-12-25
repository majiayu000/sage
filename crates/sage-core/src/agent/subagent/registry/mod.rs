//! Agent registry for managing available subagents and running agent instances

mod definitions;
mod running;
#[cfg(test)]
mod tests;
mod types;

// Re-export the main type
pub use types::AgentRegistry;

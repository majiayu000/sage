//! Sub-agent executor for running agents with filtered tools

mod executor;
mod handlers;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use executor::SubAgentExecutor;
pub use types::{AgentProgress, ExecutorMessage, SubAgentConfig};

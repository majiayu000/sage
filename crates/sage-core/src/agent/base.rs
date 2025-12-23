//! Base agent implementation

mod agent_impl;
mod continue_execution;
mod execution_loop;
mod llm_interaction;
mod messages;
mod model_identity;
mod step_execution;
mod system_prompt;
mod tool_display;
mod tool_execution;
mod trait_impl;
mod utils;

// Re-export the trait
pub use self::r#trait::Agent;

// Re-export BaseAgent
pub use agent_impl::BaseAgent;

// Re-export utility functions
pub use utils::is_markdown_content;

// Internal trait module
mod r#trait;

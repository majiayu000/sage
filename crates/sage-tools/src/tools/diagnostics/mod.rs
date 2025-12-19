//! Diagnostics and content processing tools

pub mod content_processing;
pub mod ide_diagnostics;
pub mod learning;
pub mod memory;
pub mod mermaid;

// Re-export tools
pub use content_processing::{SearchUntruncatedTool, ViewRangeUntruncatedTool};
pub use ide_diagnostics::DiagnosticsTool;
pub use learning::{
    get_global_learning_engine, get_learning_patterns_for_context, init_global_learning_engine,
    LearnTool, LearningPatternsTool,
};
pub use memory::{
    get_global_memory_manager, get_memories_for_context, init_global_memory_manager,
    RememberTool, SessionNotesTool,
};
pub use mermaid::RenderMermaidTool;

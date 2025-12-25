//! Tool implementations for Sage Agent
//!
//! This crate provides a comprehensive collection of tools for the Sage Agent system,
//! organized into functional categories for code manipulation, process execution,
//! file operations, and more.
//!
//! # Tool Categories
//!
//! ## File Operations ([`tools::file_ops`])
//! - **Read** - Read file contents with line numbers
//! - **Write** - Create or overwrite files
//! - **Edit** - Precise text replacement in files
//! - **Glob** - Pattern-based file discovery
//! - **Grep** - Content search with regex support
//! - **MultiEdit** - Batch file editing operations
//! - **JsonEdit** - JSON file manipulation with JSONPath
//!
//! ## Process Execution ([`tools::process`])
//! - **Bash** - Shell command execution with sandboxing
//! - **TaskOutput** - Retrieve background task results
//! - **KillShell** - Terminate running shell processes
//!
//! ## Code Intelligence
//! - **LSP** - Language Server Protocol integration (in [`tools::file_ops`])
//! - **TestGenerator** - Automated test generation (in [`tools::file_ops`])
//! - **CodebaseRetrieval** - Semantic code search (in [`tools::file_ops`])
//!
//! ## Network Operations ([`tools::network`])
//! - **WebFetch** - HTTP content fetching
//! - **WebSearch** - Web search integration
//! - **HttpClient** - Full-featured HTTP client
//!
//! ## Diagnostics ([`tools::diagnostics`])
//! - **Learning** - Pattern learning from user interactions
//! - **Memory** - Long-term memory management
//!
//! ## Extensions ([`tools::extensions`])
//! - **Skill** - Skill invocation
//! - **SlashCommand** - Slash command handling
//!
//! # Usage
//!
//! ```rust,ignore
//! use sage_tools::get_default_tools;
//!
//! // Get all default tools
//! let tools = get_default_tools();
//! ```

// Allow common clippy lints that are stylistic preferences
#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::manual_range_patterns)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::filter_map_identity)]

pub mod config;
pub mod mcp_tools;
pub mod tools;

// Re-export tools from the organized structure
pub use tools::*;

use sage_core::tools::Tool;
use std::sync::Arc;

/// Get all default tools
pub fn get_default_tools() -> Vec<Arc<dyn Tool>> {
    tools::get_default_tools()
}

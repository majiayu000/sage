//! Tool implementations for Sage Agent

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

//! Task execution module

mod run;
mod unified;

use sage_core::tools::Tool;
use std::sync::Arc;

#[cfg(feature = "default-tools")]
pub(super) fn default_tools() -> Vec<Arc<dyn Tool>> {
    sage_tools::get_default_tools()
}

#[cfg(not(feature = "default-tools"))]
pub(super) fn default_tools() -> Vec<Arc<dyn Tool>> {
    Vec::new()
}

//! Enhanced message types - re-exported from the unified session data model.
//!
//! The canonical definitions live in `crate::session::types::unified`.
//! Only `MessageContent` is re-exported here since it shares the same name.

use crate::session::types::unified;

/// `MessageContent` re-exported from the unified module.
pub type MessageContent = unified::MessageContent;

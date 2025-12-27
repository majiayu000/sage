//! Session management and persistence for Sage Agent
//!
//! This crate provides session management capabilities including:
//! - Session creation and lifecycle management
//! - Message history persistence
//! - Session metadata and filtering
//! - Local file storage

pub mod session;
pub mod storage;

pub use session::{Message, Role, Session, SessionMetadata, Summary};
pub use storage::{LocalSessionStorage, SessionFilter, SessionStorage, StorageError};

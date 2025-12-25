//! Database backend implementations
//!
//! Provides a unified interface for different database backends with
//! SQLite and PostgreSQL implementations.

mod postgres;
mod sqlite;
#[allow(clippy::module_inception)]
mod r#trait;
mod types;

// Re-export all public APIs
pub use postgres::PostgresBackend;
pub use sqlite::SqliteBackend;
pub use r#trait::DatabaseBackend;
pub use types::{BackendType, DatabaseError, DatabaseRow, DatabaseValue, QueryResult};

//! Database Tools
//!
//! This module provides tools for interacting with various database systems,
//! including relational databases and NoSQL databases.

pub mod sql;
pub mod mongodb;

pub use sql::DatabaseTool;
pub use mongodb::MongoDbTool;

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all database tools
pub fn get_database_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(DatabaseTool::new()),
        Arc::new(MongoDbTool::new()),
    ]
}
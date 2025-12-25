//! DDL operation handlers (CREATE TABLE, DROP TABLE, CREATE INDEX)

use tracing::info;
use crate::tools::database::sql::types::{ColumnDefinition, QueryResult};

pub fn execute_create_table(
    table: String,
    columns: Vec<ColumnDefinition>,
    start_time: std::time::Instant,
) -> QueryResult {
    info!("Creating table: {} with {} columns", table, columns.len());
    QueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

pub fn execute_drop_table(table: String, start_time: std::time::Instant) -> QueryResult {
    info!("Dropping table: {}", table);
    QueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

pub fn execute_create_index(
    table: String,
    index_name: String,
    columns: Vec<String>,
    unique: bool,
    start_time: std::time::Instant,
) -> QueryResult {
    info!(
        "Creating {} index {} on table {} for columns: {}",
        if unique { "unique" } else { "non-unique" },
        index_name,
        table,
        columns.join(", ")
    );
    QueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

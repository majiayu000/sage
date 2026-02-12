//! DDL operation handlers (CREATE TABLE, DROP TABLE, CREATE INDEX)

use tracing::info;
use crate::tools::database::sql::types::{ColumnDefinition, SqlQueryResult};

pub fn execute_create_table(
    table: String,
    columns: Vec<ColumnDefinition>,
    start_time: std::time::Instant,
) -> SqlQueryResult {
    info!("Creating table: {} with {} columns", table, columns.len());
    SqlQueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

pub fn execute_drop_table(table: String, start_time: std::time::Instant) -> SqlQueryResult {
    info!("Dropping table: {}", table);
    SqlQueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

pub fn execute_create_index(
    table: String,
    index_name: String,
    columns: Vec<String>,
    unique: bool,
    start_time: std::time::Instant,
) -> SqlQueryResult {
    info!(
        "Creating {} index {} on table {} for columns: {}",
        if unique { "unique" } else { "non-unique" },
        index_name,
        table,
        columns.join(", ")
    );
    SqlQueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

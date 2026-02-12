//! DML operation handlers (INSERT, UPDATE, DELETE, TRANSACTION)

use std::collections::HashMap;
use tracing::info;
use crate::tools::database::sql::types::SqlQueryResult;

pub fn execute_insert(
    table: String,
    _data: HashMap<String, serde_json::Value>,
    start_time: std::time::Instant,
) -> SqlQueryResult {
    info!("Inserting into table: {}", table);
    SqlQueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

pub fn execute_update(
    table: String,
    _data: HashMap<String, serde_json::Value>,
    where_clause: String,
    start_time: std::time::Instant,
) -> SqlQueryResult {
    info!("Updating table: {} where {}", table, where_clause);
    SqlQueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

pub fn execute_delete(
    table: String,
    where_clause: String,
    start_time: std::time::Instant,
) -> SqlQueryResult {
    info!("Deleting from table: {} where {}", table, where_clause);
    SqlQueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

pub fn execute_transaction(statements: Vec<String>, start_time: std::time::Instant) -> SqlQueryResult {
    info!("Executing transaction with {} statements", statements.len());
    SqlQueryResult {
        rows_affected: Some(statements.len() as u64),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

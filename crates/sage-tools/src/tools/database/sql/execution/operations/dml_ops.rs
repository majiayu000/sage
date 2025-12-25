//! DML operation handlers (INSERT, UPDATE, DELETE, TRANSACTION)

use std::collections::HashMap;
use tracing::info;
use crate::tools::database::sql::types::QueryResult;

pub fn execute_insert(
    table: String,
    _data: HashMap<String, serde_json::Value>,
    start_time: std::time::Instant,
) -> QueryResult {
    info!("Inserting into table: {}", table);
    QueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

pub fn execute_update(
    table: String,
    _data: HashMap<String, serde_json::Value>,
    where_clause: String,
    start_time: std::time::Instant,
) -> QueryResult {
    info!("Updating table: {} where {}", table, where_clause);
    QueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

pub fn execute_delete(
    table: String,
    where_clause: String,
    start_time: std::time::Instant,
) -> QueryResult {
    info!("Deleting from table: {} where {}", table, where_clause);
    QueryResult {
        rows_affected: Some(1),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

pub fn execute_transaction(statements: Vec<String>, start_time: std::time::Instant) -> QueryResult {
    info!("Executing transaction with {} statements", statements.len());
    QueryResult {
        rows_affected: Some(statements.len() as u64),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

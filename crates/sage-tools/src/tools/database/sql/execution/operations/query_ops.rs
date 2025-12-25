//! Query operation handlers (SELECT, raw queries)

use std::collections::HashMap;
use tracing::info;
use crate::tools::database::sql::types::QueryResult;

pub fn execute_query(sql: String, start_time: std::time::Instant) -> QueryResult {
    info!("Executing query: {}", sql);
    QueryResult {
        rows_affected: Some(0),
        data: None,
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: None,
    }
}

pub fn execute_select(
    sql: String,
    limit: Option<usize>,
    start_time: std::time::Instant,
) -> QueryResult {
    info!("Executing select: {}", sql);
    // Mock data
    let mut data = vec![
        HashMap::from([
            ("id".to_string(), serde_json::json!(1)),
            ("name".to_string(), serde_json::json!("Sample Record")),
        ]),
        HashMap::from([
            ("id".to_string(), serde_json::json!(2)),
            ("name".to_string(), serde_json::json!("Another Record")),
        ]),
    ];

    if let Some(limit) = limit {
        data.truncate(limit);
    }

    QueryResult {
        rows_affected: Some(data.len() as u64),
        data: Some(data),
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: Some(vec!["id".to_string(), "name".to_string()]),
    }
}

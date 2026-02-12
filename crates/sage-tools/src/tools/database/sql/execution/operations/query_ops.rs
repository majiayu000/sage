//! Query operation handlers (SELECT, raw queries)

use std::collections::HashMap;
use tracing::info;
use crate::tools::database::sql::types::SqlQueryResult;

pub fn execute_query(sql: String, start_time: std::time::Instant) -> SqlQueryResult {
    info!("Executing query: {}", sql);
    SqlQueryResult {
        rows_affected: Some(0),
        data: None,
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: None,
    }
}

pub fn execute_select(
    sql: String,
    limit: Option<usize>,
    start_time: std::time::Instant,
) -> SqlQueryResult {
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

    SqlQueryResult {
        rows_affected: Some(data.len() as u64),
        data: Some(data),
        execution_time: u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        columns: Some(vec!["id".to_string(), "name".to_string()]),
    }
}

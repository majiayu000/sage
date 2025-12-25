//! Utility operation handlers (DESCRIBE, LIST TABLES, STATS)

use std::collections::HashMap;
use tracing::info;
use crate::tools::database::sql::types::QueryResult;

pub fn execute_describe_table(table: String, start_time: std::time::Instant) -> QueryResult {
    info!("Describing table: {}", table);
    let schema_data = vec![
        HashMap::from([
            ("column_name".to_string(), serde_json::json!("id")),
            ("data_type".to_string(), serde_json::json!("INTEGER")),
            ("nullable".to_string(), serde_json::json!(false)),
            ("primary_key".to_string(), serde_json::json!(true)),
        ]),
        HashMap::from([
            ("column_name".to_string(), serde_json::json!("name")),
            ("data_type".to_string(), serde_json::json!("VARCHAR(255)")),
            ("nullable".to_string(), serde_json::json!(true)),
            ("primary_key".to_string(), serde_json::json!(false)),
        ]),
    ];

    QueryResult {
        rows_affected: Some(schema_data.len() as u64),
        data: Some(schema_data),
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: Some(vec![
            "column_name".to_string(),
            "data_type".to_string(),
            "nullable".to_string(),
            "primary_key".to_string(),
        ]),
    }
}

pub fn execute_list_tables(start_time: std::time::Instant) -> QueryResult {
    info!("Listing tables");
    let tables_data = vec![
        HashMap::from([("table_name".to_string(), serde_json::json!("users"))]),
        HashMap::from([("table_name".to_string(), serde_json::json!("orders"))]),
        HashMap::from([("table_name".to_string(), serde_json::json!("products"))]),
    ];

    QueryResult {
        rows_affected: Some(tables_data.len() as u64),
        data: Some(tables_data),
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: Some(vec!["table_name".to_string()]),
    }
}

pub fn execute_stats(start_time: std::time::Instant) -> QueryResult {
    info!("Getting database statistics");
    let stats_data = vec![
        HashMap::from([
            ("metric".to_string(), serde_json::json!("total_tables")),
            ("value".to_string(), serde_json::json!(3)),
        ]),
        HashMap::from([
            ("metric".to_string(), serde_json::json!("total_rows")),
            ("value".to_string(), serde_json::json!(1000)),
        ]),
        HashMap::from([
            ("metric".to_string(), serde_json::json!("database_size")),
            ("value".to_string(), serde_json::json!("10.5 MB")),
        ]),
    ];

    QueryResult {
        rows_affected: Some(stats_data.len() as u64),
        data: Some(stats_data),
        execution_time: start_time.elapsed().as_millis() as u64,
        columns: Some(vec!["metric".to_string(), "value".to_string()]),
    }
}

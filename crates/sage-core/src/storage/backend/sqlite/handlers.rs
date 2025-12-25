//! SQL query handlers for SQLite backend
//!
//! Provides parsing and handling for different SQL statement types.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::storage::backend::types::{DatabaseError, DatabaseRow, DatabaseValue, QueryResult};

/// SQL query handler for in-memory database operations
pub(super) struct QueryHandler {
    pub(super) data: Arc<RwLock<HashMap<String, Vec<DatabaseRow>>>>,
}

impl QueryHandler {
    /// Extract column names from INSERT statement
    /// Parses: INSERT INTO table (col1, col2, col3) VALUES (?, ?, ?)
    pub(super) fn extract_insert_columns(&self, sql: &str) -> Vec<String> {
        // Find the column list between first ( and ) before VALUES
        let sql_lower = sql.to_lowercase();

        // Find the opening parenthesis after table name
        if let Some(open_paren) = sql.find('(') {
            // Find VALUES keyword
            let values_pos = sql_lower.find("values").unwrap_or(sql.len());

            // The column list should be between first ( and the ) before VALUES
            if open_paren < values_pos {
                // Find closing paren for column list
                let after_open = &sql[open_paren + 1..];
                if let Some(close_paren) = after_open.find(')') {
                    let column_str = &after_open[..close_paren];

                    // Parse column names
                    return column_str
                        .split(',')
                        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '`').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }

        Vec::new()
    }

    /// Handle SELECT queries
    pub(super) async fn handle_query(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        let sql_lower = sql.to_lowercase();
        if sql_lower.contains("select") && sql_lower.contains("from") {
            // Extract table name
            if let Some(from_pos) = sql_lower.find("from") {
                let after_from = &sql_lower[from_pos + 5..];
                let table_name = after_from
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '"' || c == '`' || c == ';');

                let data = self.data.read().await;
                if let Some(rows) = data.get(table_name) {
                    // Check for WHERE clause with key parameter
                    if sql_lower.contains("where") && !params.is_empty() {
                        // Find rows matching the key (first param)
                        if let DatabaseValue::Text(key) = &params[0] {
                            let matching: Vec<DatabaseRow> = rows
                                .iter()
                                .filter(|r| {
                                    r.get("key").and_then(|v| v.as_str()) == Some(key.as_str())
                                })
                                .cloned()
                                .collect();
                            return Ok(QueryResult::from_rows(matching));
                        }
                    }

                    // Check for ORDER BY ... LIMIT 1 (for schema version)
                    if sql_lower.contains("limit 1") {
                        if let Some(first) = rows.last() {
                            return Ok(QueryResult::from_rows(vec![first.clone()]));
                        }
                    }

                    return Ok(QueryResult::from_rows(rows.clone()));
                }
            }
        }

        Ok(QueryResult::empty())
    }

    /// Handle CREATE TABLE statements
    pub(super) async fn handle_create_table(
        &self,
        sql: &str,
    ) -> Result<QueryResult, DatabaseError> {
        // Extract table name
        if let Some(start) = sql.to_lowercase().find("create table") {
            let after = &sql[start + 12..];
            let table_name = after
                .trim()
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '"' || c == '`')
                .to_string();

            if !table_name.is_empty() {
                let mut data = self.data.write().await;
                data.entry(table_name).or_insert_with(Vec::new);
            }
        }
        Ok(QueryResult::from_affected(0))
    }

    /// Handle INSERT statements
    pub(super) async fn handle_insert(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        let sql_lower = sql.to_lowercase();
        // Extract table name and column names
        if let Some(into_pos) = sql_lower.find("into") {
            let after_into = &sql[into_pos + 4..];
            let table_name = after_into
                .trim()
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '"' || c == '`')
                .to_string();

            if !table_name.is_empty() {
                // Extract column names from INSERT INTO table (col1, col2, ...) VALUES (?, ?)
                let column_names = self.extract_insert_columns(sql);

                let mut data = self.data.write().await;
                let table = data.entry(table_name).or_insert_with(Vec::new);

                // Create row from params with proper column names
                let mut row = DatabaseRow::new();
                for (i, param) in params.iter().enumerate() {
                    let col_name = column_names.get(i).map(|s| s.as_str()).unwrap_or_else(|| {
                        // Fallback to generic names
                        match i {
                            0 => "col0",
                            1 => "col1",
                            2 => "col2",
                            3 => "col3",
                            _ => "col_unknown",
                        }
                    });
                    row.set(col_name, param.clone());
                }
                table.push(row);

                return Ok(QueryResult {
                    rows_affected: 1,
                    rows: Vec::new(),
                    last_insert_id: Some(table.len() as i64),
                });
            }
        }

        Ok(QueryResult::from_affected(0))
    }

    /// Handle DELETE statements
    pub(super) async fn handle_delete(
        &self,
        sql: &str,
        params: &[DatabaseValue],
    ) -> Result<QueryResult, DatabaseError> {
        let sql_lower = sql.to_lowercase();
        if let Some(from_pos) = sql_lower.find("from") {
            let after_from = &sql[from_pos + 4..];
            let table_name = after_from
                .trim()
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '"' || c == '`')
                .to_string();

            if !table_name.is_empty() {
                let mut data = self.data.write().await;
                if let Some(table) = data.get_mut(&table_name) {
                    // Check for WHERE clause with key
                    if sql_lower.contains("where") && !params.is_empty() {
                        if let DatabaseValue::Text(key) = &params[0] {
                            let original_len = table.len();
                            table.retain(|row| {
                                row.get("key").and_then(|v| v.as_str()) != Some(key.as_str())
                            });
                            let deleted = original_len - table.len();
                            return Ok(QueryResult::from_affected(deleted as u64));
                        }
                        // Also check for version column (for schema_migrations)
                        if let DatabaseValue::Int(version) = &params[0] {
                            let original_len = table.len();
                            table.retain(|row| {
                                row.get("version").and_then(|v| v.as_i64()) != Some(*version)
                            });
                            let deleted = original_len - table.len();
                            return Ok(QueryResult::from_affected(deleted as u64));
                        }
                    }
                }
            }
        }

        Ok(QueryResult::from_affected(0))
    }
}

//! Individual operation execution handlers

mod query_ops;
mod dml_ops;
mod ddl_ops;
mod utility_ops;

use anyhow::Result;
use crate::tools::database::sql::types::{DatabaseOperation, SqlQueryResult};

/// Execute individual database operations
pub async fn execute_operation_internal(
    operation: DatabaseOperation,
    start_time: std::time::Instant,
) -> Result<SqlQueryResult> {
    let result = match operation {
        DatabaseOperation::Query { sql, params: _ } => {
            query_ops::execute_query(sql, start_time)
        }
        DatabaseOperation::Select { sql, params: _, limit } => {
            query_ops::execute_select(sql, limit, start_time)
        }
        DatabaseOperation::Insert { table, data } => {
            dml_ops::execute_insert(table, data, start_time)
        }
        DatabaseOperation::Update { table, data, where_clause, params: _ } => {
            dml_ops::execute_update(table, data, where_clause, start_time)
        }
        DatabaseOperation::Delete { table, where_clause, params: _ } => {
            dml_ops::execute_delete(table, where_clause, start_time)
        }
        DatabaseOperation::Transaction { statements } => {
            dml_ops::execute_transaction(statements, start_time)
        }
        DatabaseOperation::CreateTable { table, columns } => {
            ddl_ops::execute_create_table(table, columns, start_time)
        }
        DatabaseOperation::DropTable { table } => {
            ddl_ops::execute_drop_table(table, start_time)
        }
        DatabaseOperation::CreateIndex { table, index_name, columns, unique } => {
            ddl_ops::execute_create_index(table, index_name, columns, unique, start_time)
        }
        DatabaseOperation::DescribeTable { table } => {
            utility_ops::execute_describe_table(table, start_time)
        }
        DatabaseOperation::ListTables => {
            utility_ops::execute_list_tables(start_time)
        }
        DatabaseOperation::Stats => {
            utility_ops::execute_stats(start_time)
        }
    };

    Ok(result)
}

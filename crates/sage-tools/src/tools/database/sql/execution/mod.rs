//! Database Operation Execution Logic

pub(crate) mod operations;
mod formatting;

use anyhow::Result;
use tracing::{info, debug};

use crate::tools::database::sql::types::{DatabaseParams, SqlQueryResult};
use crate::tools::database::sql::validation::build_connection_string;

pub use formatting::format_result;

/// Execute a database operation (mock implementation)
pub async fn execute_operation(params: DatabaseParams) -> Result<SqlQueryResult> {
    let start_time = std::time::Instant::now();

    debug!("Executing database operation: {:?}", params.operation);

    // This is a mock implementation. In a real implementation, you would:
    // 1. Create a connection pool using sqlx or similar
    // 2. Execute the actual SQL operations
    // 3. Return real results

    let connection_string = build_connection_string(&params.config)?;
    let redacted = connection_string
        .find("://")
        .and_then(|scheme_end| {
            let after_scheme = &connection_string[scheme_end + 3..];
            after_scheme.find('@').map(|at_pos| {
                format!(
                    "{}://***@{}",
                    &connection_string[..scheme_end],
                    &after_scheme[at_pos + 1..]
                )
            })
        })
        .unwrap_or_else(|| "***".to_string());
    info!("Connecting to database: {}", redacted);

    operations::execute_operation_internal(params.operation, start_time).await
}

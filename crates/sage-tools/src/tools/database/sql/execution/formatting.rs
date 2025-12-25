//! Result formatting for display

use crate::tools::database::sql::types::QueryResult;

/// Format query result for display
pub fn format_result(result: &QueryResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Execution time: {}ms\n", result.execution_time));

    if let Some(rows_affected) = result.rows_affected {
        output.push_str(&format!("Rows affected: {}\n", rows_affected));
    }

    if let Some(data) = &result.data {
        if !data.is_empty() {
            output.push_str("\nResults:\n");

            // Get column names
            let columns = if let Some(cols) = &result.columns {
                cols.clone()
            } else {
                // Extract column names from first row
                data.first()
                    .map(|row| row.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            };

            // Print header
            output.push_str(&format!("| {} |\n", columns.join(" | ")));
            output.push_str(&format!(
                "|{}|\n",
                columns.iter().map(|_| "---").collect::<Vec<_>>().join("|")
            ));

            // Print rows
            for row in data {
                let values: Vec<String> = columns
                    .iter()
                    .map(|col| {
                        row.get(col)
                            .map(|v| v.to_string().trim_matches('"').to_string())
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();
                output.push_str(&format!("| {} |\n", values.join(" | ")));
            }
        }
    }

    output
}

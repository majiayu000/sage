//! Test data generation

use tracing::debug;

use sage_core::tools::base::ToolError;

use super::types::TestGeneratorTool;

impl TestGeneratorTool {
    /// Generate test data
    pub(super) async fn generate_test_data(
        &self,
        data_type: &str,
        format: &str,
    ) -> Result<String, ToolError> {
        debug!("Generating test data of type: {}", data_type);

        let test_data = match data_type {
            "user" => match format {
                "json" => r#"{
    "users": [
        {
            "id": 1,
            "name": "John Doe",
            "email": "john.doe@example.com",
            "age": 30,
            "active": true,
            "created_at": "2024-01-01T00:00:00Z"
        },
        {
            "id": 2,
            "name": "Jane Smith",
            "email": "jane.smith@example.com",
            "age": 25,
            "active": false,
            "created_at": "2024-01-02T00:00:00Z"
        }
    ]
}"#
                .to_string(),
                "csv" => r#"id,name,email,age,active,created_at
1,John Doe,john.doe@example.com,30,true,2024-01-01T00:00:00Z
2,Jane Smith,jane.smith@example.com,25,false,2024-01-02T00:00:00Z
3,Bob Johnson,bob.johnson@example.com,35,true,2024-01-03T00:00:00Z"#
                    .to_string(),
                _ => {
                    "// Test user data\n// TODO: Generate data in the requested format".to_string()
                }
            },
            "product" => match format {
                "json" => r#"{
    "products": [
        {
            "id": "PROD001",
            "name": "Laptop Computer",
            "price": 999.99,
            "category": "Electronics",
            "in_stock": true,
            "quantity": 50
        },
        {
            "id": "PROD002",
            "name": "Office Chair",
            "price": 299.99,
            "category": "Furniture",
            "in_stock": false,
            "quantity": 0
        }
    ]
}"#
                .to_string(),
                _ => "// Test product data\n// TODO: Generate data in the requested format"
                    .to_string(),
            },
            _ => format!(
                "// Test data for {}\n// TODO: Generate specific test data",
                data_type
            ),
        };

        Ok(format!(
            "Generated test data for type '{}':\n{}",
            data_type, test_data
        ))
    }
}

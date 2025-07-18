//! MongoDB Tool
//!
//! This tool provides MongoDB operations including:
//! - Document CRUD operations
//! - Collection management
//! - Database administration
//! - Query operations
//! - Aggregation pipelines

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, debug};

use sage_core::tools::{Tool, ToolResult};

/// MongoDB operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MongoOperation {
    /// Find documents
    Find {
        collection: String,
        filter: serde_json::Value,
        limit: Option<i64>,
        sort: Option<serde_json::Value>,
    },
    /// Insert a document
    InsertOne {
        collection: String,
        document: serde_json::Value,
    },
    /// Insert multiple documents
    InsertMany {
        collection: String,
        documents: Vec<serde_json::Value>,
    },
    /// Update a document
    UpdateOne {
        collection: String,
        filter: serde_json::Value,
        update: serde_json::Value,
    },
    /// Update multiple documents
    UpdateMany {
        collection: String,
        filter: serde_json::Value,
        update: serde_json::Value,
    },
    /// Delete a document
    DeleteOne {
        collection: String,
        filter: serde_json::Value,
    },
    /// Delete multiple documents
    DeleteMany {
        collection: String,
        filter: serde_json::Value,
    },
    /// Count documents
    CountDocuments {
        collection: String,
        filter: serde_json::Value,
    },
    /// List collections
    ListCollections,
    /// Create collection
    CreateCollection {
        name: String,
    },
    /// Drop collection
    DropCollection {
        name: String,
    },
    /// Aggregate
    Aggregate {
        collection: String,
        pipeline: Vec<serde_json::Value>,
    },
    /// Database stats
    DbStats,
}

/// MongoDB tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoDbParams {
    /// MongoDB connection string
    pub connection_string: String,
    /// Database name
    pub database: String,
    /// MongoDB operation
    pub operation: MongoOperation,
}

/// MongoDB tool
#[derive(Debug, Clone)]
pub struct MongoDbTool {
    name: String,
    description: String,
}

impl MongoDbTool {
    /// Create a new MongoDB tool
    pub fn new() -> Self {
        Self {
            name: "mongodb".to_string(),
            description: "MongoDB operations including document CRUD, collection management, and aggregation".to_string(),
        }
    }

    /// Execute MongoDB operation (mock implementation)
    async fn execute_operation(&self, params: MongoDbParams) -> Result<serde_json::Value> {
        debug!("Executing MongoDB operation: {:?}", params.operation);
        
        // Mock implementation - in reality, you would use the MongoDB driver
        let result = match params.operation {
            MongoOperation::Find { collection, filter, limit, sort } => {
                info!("Finding documents in collection: {}", collection);
                serde_json::json!({
                    "documents": [
                        {"_id": "507f1f77bcf86cd799439011", "name": "Sample Document", "value": 42},
                        {"_id": "507f1f77bcf86cd799439012", "name": "Another Document", "value": 84}
                    ],
                    "count": 2
                })
            }
            MongoOperation::InsertOne { collection, document } => {
                info!("Inserting document into collection: {}", collection);
                serde_json::json!({
                    "inserted_id": "507f1f77bcf86cd799439013",
                    "acknowledged": true
                })
            }
            MongoOperation::InsertMany { collection, documents } => {
                info!("Inserting {} documents into collection: {}", documents.len(), collection);
                serde_json::json!({
                    "inserted_ids": ["507f1f77bcf86cd799439014", "507f1f77bcf86cd799439015"],
                    "acknowledged": true
                })
            }
            MongoOperation::UpdateOne { collection, filter, update } => {
                info!("Updating document in collection: {}", collection);
                serde_json::json!({
                    "matched_count": 1,
                    "modified_count": 1,
                    "acknowledged": true
                })
            }
            MongoOperation::UpdateMany { collection, filter, update } => {
                info!("Updating documents in collection: {}", collection);
                serde_json::json!({
                    "matched_count": 2,
                    "modified_count": 2,
                    "acknowledged": true
                })
            }
            MongoOperation::DeleteOne { collection, filter } => {
                info!("Deleting document from collection: {}", collection);
                serde_json::json!({
                    "deleted_count": 1,
                    "acknowledged": true
                })
            }
            MongoOperation::DeleteMany { collection, filter } => {
                info!("Deleting documents from collection: {}", collection);
                serde_json::json!({
                    "deleted_count": 3,
                    "acknowledged": true
                })
            }
            MongoOperation::CountDocuments { collection, filter } => {
                info!("Counting documents in collection: {}", collection);
                serde_json::json!({
                    "count": 5
                })
            }
            MongoOperation::ListCollections => {
                info!("Listing collections");
                serde_json::json!({
                    "collections": [
                        {"name": "users", "type": "collection"},
                        {"name": "orders", "type": "collection"},
                        {"name": "products", "type": "collection"}
                    ]
                })
            }
            MongoOperation::CreateCollection { name } => {
                info!("Creating collection: {}", name);
                serde_json::json!({
                    "ok": 1,
                    "collection": name
                })
            }
            MongoOperation::DropCollection { name } => {
                info!("Dropping collection: {}", name);
                serde_json::json!({
                    "ok": 1,
                    "dropped": name
                })
            }
            MongoOperation::Aggregate { collection, pipeline } => {
                info!("Running aggregation pipeline on collection: {}", collection);
                serde_json::json!({
                    "results": [
                        {"_id": "category1", "count": 10, "total": 500},
                        {"_id": "category2", "count": 15, "total": 750}
                    ]
                })
            }
            MongoOperation::DbStats => {
                info!("Getting database statistics");
                serde_json::json!({
                    "db": params.database,
                    "collections": 3,
                    "objects": 1000,
                    "avgObjSize": 250.5,
                    "dataSize": 250500,
                    "storageSize": 512000,
                    "numExtents": 10,
                    "indexes": 5,
                    "indexSize": 50000,
                    "ok": 1
                })
            }
        };
        
        Ok(result)
    }
}

impl Default for MongoDbTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MongoDbTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "connection_string": {
                    "type": "string",
                    "description": "MongoDB connection string"
                },
                "database": {
                    "type": "string",
                    "description": "Database name"
                },
                "operation": {
                    "type": "object",
                    "oneOf": [
                        {
                            "properties": {
                                "find": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "filter": { "type": "object" },
                                        "limit": { "type": "integer" },
                                        "sort": { "type": "object" }
                                    },
                                    "required": ["collection", "filter"]
                                }
                            },
                            "required": ["find"]
                        },
                        {
                            "properties": {
                                "insert_one": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "document": { "type": "object" }
                                    },
                                    "required": ["collection", "document"]
                                }
                            },
                            "required": ["insert_one"]
                        },
                        {
                            "properties": {
                                "insert_many": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "documents": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        }
                                    },
                                    "required": ["collection", "documents"]
                                }
                            },
                            "required": ["insert_many"]
                        },
                        {
                            "properties": {
                                "update_one": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "filter": { "type": "object" },
                                        "update": { "type": "object" }
                                    },
                                    "required": ["collection", "filter", "update"]
                                }
                            },
                            "required": ["update_one"]
                        },
                        {
                            "properties": {
                                "update_many": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "filter": { "type": "object" },
                                        "update": { "type": "object" }
                                    },
                                    "required": ["collection", "filter", "update"]
                                }
                            },
                            "required": ["update_many"]
                        },
                        {
                            "properties": {
                                "delete_one": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "filter": { "type": "object" }
                                    },
                                    "required": ["collection", "filter"]
                                }
                            },
                            "required": ["delete_one"]
                        },
                        {
                            "properties": {
                                "delete_many": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "filter": { "type": "object" }
                                    },
                                    "required": ["collection", "filter"]
                                }
                            },
                            "required": ["delete_many"]
                        },
                        {
                            "properties": {
                                "count_documents": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "filter": { "type": "object" }
                                    },
                                    "required": ["collection", "filter"]
                                }
                            },
                            "required": ["count_documents"]
                        },
                        {
                            "properties": {
                                "list_collections": { "type": "null" }
                            },
                            "required": ["list_collections"]
                        },
                        {
                            "properties": {
                                "create_collection": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["create_collection"]
                        },
                        {
                            "properties": {
                                "drop_collection": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["drop_collection"]
                        },
                        {
                            "properties": {
                                "aggregate": {
                                    "type": "object",
                                    "properties": {
                                        "collection": { "type": "string" },
                                        "pipeline": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        }
                                    },
                                    "required": ["collection", "pipeline"]
                                }
                            },
                            "required": ["aggregate"]
                        },
                        {
                            "properties": {
                                "db_stats": { "type": "null" }
                            },
                            "required": ["db_stats"]
                        }
                    ]
                }
            },
            "required": ["connection_string", "database", "operation"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: MongoDbParams = serde_json::from_value(params)
            .context("Failed to parse MongoDB parameters")?;

        info!("Executing MongoDB operation on database: {}", params.database);

        let result = self.execute_operation(params).await?;
        let formatted_result = serde_json::to_string_pretty(&result)?;
        
        let metadata = HashMap::new();

        Ok(ToolResult::new(formatted_result, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mongodb_tool_creation() {
        let tool = MongoDbTool::new();
        assert_eq!(tool.name(), "mongodb");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_mongodb_tool_schema() {
        let tool = MongoDbTool::new();
        let schema = tool.parameters_json_schema();
        
        assert!(schema.is_object());
        assert!(schema["properties"]["connection_string"].is_object());
        assert!(schema["properties"]["database"].is_object());
        assert!(schema["properties"]["operation"].is_object());
    }
}
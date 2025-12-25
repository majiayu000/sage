//! Email tool implementation
//!
//! This module provides the EmailTool struct and Tool trait implementation.

use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, Context};
use tracing::info;

use sage_core::tools::{Tool, ToolResult};

use crate::tools::data::email::types::{EmailOperation, EmailParams};
use crate::tools::data::email::schema::get_email_schema;
use crate::tools::data::email::sender;

/// Email tool
#[derive(Debug, Clone)]
pub struct EmailTool {
    name: String,
    description: String,
}

impl EmailTool {
    /// Create a new email tool
    pub fn new() -> Self {
        Self {
            name: "email".to_string(),
            description: "Email operations including SMTP sending, IMAP reading, template processing, and email validation".to_string(),
        }
    }

    /// Execute email operation
    async fn execute_operation(&self, operation: EmailOperation) -> Result<serde_json::Value> {
        match operation {
            EmailOperation::Send { smtp_config, message } => {
                sender::send_email(&smtp_config, &message).await
            }
            EmailOperation::Read { imap_config, folder, limit, unread_only } => {
                sender::read_emails(&imap_config, folder.as_deref(), limit, unread_only).await
            }
            EmailOperation::ValidateEmail { email } => {
                sender::validate_email(&email).await
            }
            EmailOperation::ProcessTemplate { template, variables } => {
                sender::process_template(&template, &variables).await
            }
            EmailOperation::ParseEmail { raw_email } => {
                sender::parse_email(&raw_email).await
            }
        }
    }
}

impl Default for EmailTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EmailTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        get_email_schema()
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: EmailParams = serde_json::from_value(params)
            .context("Failed to parse email parameters")?;

        info!("Executing email operation: {:?}", params.operation);

        let result = self.execute_operation(params.operation).await?;
        let formatted_result = serde_json::to_string_pretty(&result)?;

        let metadata = HashMap::new();

        Ok(ToolResult::new(formatted_result, metadata))
    }
}

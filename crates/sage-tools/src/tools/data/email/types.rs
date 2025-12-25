//! Email type definitions
//!
//! This module contains core types for email operations.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Email operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailOperation {
    /// Send email
    Send {
        smtp_config: SmtpConfig,
        message: EmailMessage,
    },
    /// Read emails
    Read {
        imap_config: ImapConfig,
        folder: Option<String>,
        limit: Option<usize>,
        unread_only: bool,
    },
    /// Validate email address
    ValidateEmail {
        email: String,
    },
    /// Process email template
    ProcessTemplate {
        template: String,
        variables: HashMap<String, serde_json::Value>,
    },
    /// Parse email
    ParseEmail {
        raw_email: String,
    },
}

/// SMTP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
    pub use_starttls: bool,
}

/// IMAP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

/// Email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Option<Vec<EmailAttachment>>,
    pub headers: Option<HashMap<String, String>>,
}

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: String, // base64 encoded
}

/// Email tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailParams {
    /// Email operation
    pub operation: EmailOperation,
}

/// Parsed email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEmail {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub subject: String,
    pub date: String,
    pub message_id: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<EmailAttachment>,
    pub headers: HashMap<String, String>,
}

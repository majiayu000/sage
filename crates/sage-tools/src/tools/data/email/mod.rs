//! Email Tool
//!
//! This tool provides email operations including:
//! - SMTP email sending
//! - IMAP email reading
//! - Email template processing
//! - Attachment handling
//! - Email validation

mod types;
mod schema;
mod sender;
mod tool;

#[cfg(test)]
mod tests;

// Re-export public types
pub use types::{
    EmailOperation,
    SmtpConfig,
    ImapConfig,
    EmailMessage,
    EmailAttachment,
    EmailParams,
    ParsedEmail,
};

// Re-export the tool
pub use tool::EmailTool;

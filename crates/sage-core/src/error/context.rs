//! Context management for SageError

use super::types::SageError;

impl SageError {
    /// Add context to any error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        let ctx = Some(context.into());
        match &mut self {
            Self::Config { context: c, .. } => *c = ctx,
            Self::Llm { context: c, .. } => *c = ctx,
            Self::Tool { context: c, .. } => *c = ctx,
            Self::Agent { context: c, .. } => *c = ctx,
            Self::Cache { context: c, .. } => *c = ctx,
            Self::Io { context: c, .. } => *c = ctx,
            Self::Json { context: c, .. } => *c = ctx,
            Self::Http { context: c, .. } => *c = ctx,
            Self::InvalidInput { context: c, .. } => *c = ctx,
            Self::Timeout { context: c, .. } => *c = ctx,
            Self::Storage { context: c, .. } => *c = ctx,
            Self::NotFound { context: c, .. } => *c = ctx,
            Self::Other { context: c, .. } => *c = ctx,
            Self::Cancelled => {}
        }
        self
    }
}

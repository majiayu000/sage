//! Error formatting utilities

use super::types::EnhancedToolError;

impl EnhancedToolError {
    /// Generate user-friendly error message
    pub fn user_friendly_message(&self) -> String {
        let mut message = String::new();

        // Add main error message
        message.push_str(&format!("❌ {}\n", self.original_error_message));

        // Add category information
        message.push_str(&format!("📂 Category: {:?}\n", self.category));

        // Add context if available
        if !self.context.is_empty() {
            message.push_str("\n📋 Context:\n");
            for (key, value) in &self.context {
                message.push_str(&format!("  • {}: {}\n", key, value));
            }
        }

        // Add suggestions if available
        if !self.suggestions.is_empty() {
            message.push_str("\n💡 Suggestions:\n");
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                message.push_str(&format!("  {}. {}\n", i + 1, suggestion));
            }
        }

        // Add recovery information
        if self.recoverable {
            message.push_str(
                "\n🔄 This error may be recoverable. Please try the suggestions above.\n",
            );
        } else {
            message.push_str("\n⚠️  This error requires manual intervention to resolve.\n");
        }

        message
    }

    /// Convert to JSON for structured logging
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": self.original_error_message,
            "error_type": self.original_error_type,
            "category": format!("{:?}", self.category),
            "recoverable": self.recoverable,
            "context": self.context,
            "suggestions": self.suggestions,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

impl std::fmt::Display for EnhancedToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.user_friendly_message())
    }
}

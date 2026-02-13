//! Error classification functions for user-friendly messages

use super::user_messages::ErrorCategory;

/// Classify configuration errors
pub(super) fn classify_config_error(message: &str) -> (String, Vec<String>) {
    let message_lower = message.to_lowercase();

    if message_lower.contains("not found") || message_lower.contains("missing") {
        (
            "Configuration file not found".to_string(),
            vec![
                "Create a sage_config.json file in the current directory".to_string(),
                "Run 'sage config init' to generate a default configuration".to_string(),
            ],
        )
    } else if message_lower.contains("invalid") || message_lower.contains("parse") {
        (
            "Invalid configuration".to_string(),
            vec![
                "Check the JSON syntax in your configuration file".to_string(),
                "Refer to sage_config.json.example for the correct format".to_string(),
            ],
        )
    } else if message_lower.contains("api_key") || message_lower.contains("apikey") {
        (
            "API key configuration issue".to_string(),
            vec![
                "Set your API key in the environment (e.g., ANTHROPIC_API_KEY)".to_string(),
                "Or add it to your configuration file".to_string(),
            ],
        )
    } else {
        (
            "Configuration error".to_string(),
            vec!["Review your configuration file for issues".to_string()],
        )
    }
}

/// Classify LLM errors
pub(super) fn classify_llm_error(
    message: &str,
    provider: Option<&str>,
) -> (ErrorCategory, String, Vec<String>) {
    let message_lower = message.to_lowercase();
    let provider_name = provider.unwrap_or("LLM provider");

    if message_lower.contains("401")
        || message_lower.contains("unauthorized")
        || message_lower.contains("invalid api key")
    {
        (
            ErrorCategory::Authentication,
            format!("{} authentication failed", provider_name),
            vec![
                "Check that your API key is correct".to_string(),
                "Ensure the API key has not expired".to_string(),
                format!("Verify the API key is for {}", provider_name),
            ],
        )
    } else if message_lower.contains("429")
        || message_lower.contains("rate limit")
        || message_lower.contains("quota")
    {
        (
            ErrorCategory::RateLimit,
            format!("{} rate limit exceeded", provider_name),
            vec![
                "Wait a moment and try again".to_string(),
                "Consider upgrading your API plan".to_string(),
                "Reduce the frequency of requests".to_string(),
            ],
        )
    } else if message_lower.contains("503") || message_lower.contains("overloaded") {
        (
            ErrorCategory::RateLimit,
            format!("{} is temporarily overloaded", provider_name),
            vec![
                "Wait a few seconds and try again".to_string(),
                "The service will automatically retry".to_string(),
            ],
        )
    } else if message_lower.contains("timeout") || message_lower.contains("timed out") {
        (
            ErrorCategory::Network,
            format!("{} request timed out", provider_name),
            vec![
                "Check your internet connection".to_string(),
                "Try again in a moment".to_string(),
            ],
        )
    } else if message_lower.contains("connection") || message_lower.contains("network") {
        (
            ErrorCategory::Network,
            format!("Cannot connect to {}", provider_name),
            vec![
                "Check your internet connection".to_string(),
                "Verify firewall settings".to_string(),
            ],
        )
    } else {
        (
            ErrorCategory::Internal,
            format!("{} error", provider_name),
            vec!["Try again or contact support if the issue persists".to_string()],
        )
    }
}

/// Classify HTTP errors
pub(super) fn classify_http_error(
    message: &str,
    status_code: Option<u16>,
    _url: Option<&str>,
) -> (ErrorCategory, String, Vec<String>) {
    match status_code {
        Some(401) => (
            ErrorCategory::Authentication,
            "Authentication required".to_string(),
            vec!["Check your API key or credentials".to_string()],
        ),
        Some(403) => (
            ErrorCategory::Authentication,
            "Access denied".to_string(),
            vec![
                "You may not have permission for this resource".to_string(),
                "Check your API key permissions".to_string(),
            ],
        ),
        Some(404) => (
            ErrorCategory::ResourceUnavailable,
            "Resource not found".to_string(),
            vec!["The requested resource does not exist".to_string()],
        ),
        Some(429) => (
            ErrorCategory::RateLimit,
            "Too many requests".to_string(),
            vec![
                "Wait a moment and try again".to_string(),
                "Consider reducing request frequency".to_string(),
            ],
        ),
        Some(500..=599) => (
            ErrorCategory::Internal,
            "Server error".to_string(),
            vec![
                "The server encountered an error".to_string(),
                "Try again in a few moments".to_string(),
            ],
        ),
        _ => {
            let message_lower = message.to_lowercase();
            if message_lower.contains("timeout") {
                (
                    ErrorCategory::Network,
                    "Request timed out".to_string(),
                    vec!["Check your internet connection".to_string()],
                )
            } else if message_lower.contains("connection") {
                (
                    ErrorCategory::Network,
                    "Connection failed".to_string(),
                    vec![
                        "Check your internet connection".to_string(),
                        "Verify the URL is correct".to_string(),
                    ],
                )
            } else {
                (
                    ErrorCategory::Network,
                    "HTTP request failed".to_string(),
                    vec!["Check network connectivity and try again".to_string()],
                )
            }
        }
    }
}

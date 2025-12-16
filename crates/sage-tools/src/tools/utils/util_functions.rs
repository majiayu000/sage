//! Utility functions for tools

/// Maximum response length for tool output (16KB)
pub const MAX_RESPONSE_LEN: usize = 16000;

/// Truncation message to append when content is clipped
pub const TRUNCATED_MESSAGE: &str = "<response clipped><NOTE>To save on context only part of this file has been shown to you. You should retry this tool after you have searched inside the file with `grep -n` in order to find the line numbers of what you are looking for.</NOTE>";

/// Truncate content if it exceeds the maximum length
pub fn maybe_truncate(content: &str) -> String {
    if content.len() <= MAX_RESPONSE_LEN {
        content.to_string()
    } else {
        format!("{}{}", &content[..MAX_RESPONSE_LEN], TRUNCATED_MESSAGE)
    }
}

/// Truncate content with custom limit
pub fn maybe_truncate_with_limit(content: &str, limit: usize) -> String {
    if content.len() <= limit {
        content.to_string()
    } else {
        format!("{}{}", &content[..limit], TRUNCATED_MESSAGE)
    }
}

/// Count approximate tokens in text (rough estimation: 1 token ≈ 4 characters)
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Truncate content based on estimated token count
pub fn maybe_truncate_by_tokens(content: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4; // Rough estimation
    maybe_truncate_with_limit(content, max_chars)
}

/// Check if a bash command might produce excessive output and suggest alternatives
pub fn check_command_efficiency(command: &str) -> Option<String> {
    let cmd = command.trim().to_lowercase();

    if cmd.starts_with("ls -r") || cmd.contains("ls -r") {
        return Some("Consider using 'str_replace_based_edit_tool' with view action for directory structure instead of 'ls -R'".to_string());
    }

    if cmd.starts_with("find .")
        && !cmd.contains("head")
        && !cmd.contains("tail")
        && !cmd.contains("-maxdepth")
    {
        return Some("Consider adding '| head -20' to limit find output, or use '-maxdepth 2' to limit depth".to_string());
    }

    if cmd.starts_with("grep -r") && !cmd.contains("head") && !cmd.contains("tail") {
        return Some(
            "Consider adding '| head -10' to limit grep output and specify target directories"
                .to_string(),
        );
    }

    if cmd.starts_with("cat ") && !cmd.contains("head") && !cmd.contains("tail") {
        return Some("Consider using 'str_replace_based_edit_tool' with view action instead of 'cat' for file reading".to_string());
    }

    None
}

/// Suggest more efficient alternatives for common inefficient commands
pub fn suggest_efficient_alternative(command: &str) -> Option<String> {
    let cmd = command.trim().to_lowercase();

    match cmd.as_str() {
        "ls -r" | "ls -r ." => Some(
            "Use: str_replace_based_edit_tool with view action on '.' for directory structure"
                .to_string(),
        ),
        cmd if cmd.starts_with("find . -name") && !cmd.contains("head") => {
            Some(format!("{} | head -20", command))
        }
        cmd if cmd.starts_with("grep -r") && !cmd.contains("head") => {
            Some(format!("{} | head -10", command))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maybe_truncate_short_content() {
        let content = "This is a short text";
        let result = maybe_truncate(content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_maybe_truncate_long_content() {
        let content = "a".repeat(MAX_RESPONSE_LEN + 100);
        let result = maybe_truncate(&content);
        assert!(result.len() > MAX_RESPONSE_LEN);
        assert!(result.contains(TRUNCATED_MESSAGE));
        assert!(result.starts_with(&"a".repeat(MAX_RESPONSE_LEN)));
    }

    #[test]
    fn test_maybe_truncate_with_limit() {
        let content = "This is a test content that is longer than the limit";
        let result = maybe_truncate_with_limit(content, 20);
        assert_eq!(result.len(), 20 + TRUNCATED_MESSAGE.len());
        assert!(result.contains(TRUNCATED_MESSAGE));
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "This is a test";
        let tokens = estimate_tokens(text);
        assert_eq!(tokens, text.len() / 4);
    }

    #[test]
    fn test_maybe_truncate_by_tokens() {
        let content = "a".repeat(1000);
        let result = maybe_truncate_by_tokens(&content, 100); // 100 tokens = ~400 chars
        assert!(result.len() <= 400 + TRUNCATED_MESSAGE.len());
    }
}

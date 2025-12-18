//! Utility functions for tools

/// Maximum response length for tool output (30KB - Claude Code compatible)
pub const MAX_RESPONSE_LEN: usize = 30000;

/// Maximum line length before truncation
pub const MAX_LINE_LENGTH: usize = 2000;

/// Truncation message to append when content is clipped
pub const TRUNCATED_MESSAGE: &str = "<response clipped><NOTE>To save on context only part of this file has been shown to you. You should retry this tool after you have searched inside the file with `grep -n` in order to find the line numbers of what you are looking for.</NOTE>";

/// Output truncation result with statistics
#[derive(Debug, Clone)]
pub struct TruncatedOutput {
    pub content: String,
    pub was_truncated: bool,
    pub original_chars: usize,
    pub original_lines: usize,
    pub truncated_chars: usize,
    pub truncated_lines: usize,
}

impl TruncatedOutput {
    /// Format the output with truncation message if needed
    pub fn format_with_message(&self) -> String {
        if !self.was_truncated {
            return self.content.clone();
        }

        let truncation_info = if self.truncated_lines > 0 {
            format!(
                "\n\n... [{} lines truncated, {} characters total] ...",
                self.truncated_lines, self.truncated_chars
            )
        } else {
            format!("\n\n... [{} characters truncated] ...", self.truncated_chars)
        };

        format!("{}{}{}", self.content, truncation_info, TRUNCATED_MESSAGE)
    }
}

/// Truncate content if it exceeds the maximum length with statistics
pub fn truncate_output(content: &str) -> TruncatedOutput {
    truncate_output_with_limit(content, MAX_RESPONSE_LEN)
}

/// Truncate content with custom limit, truncating long lines first
pub fn truncate_output_with_limit(content: &str, limit: usize) -> TruncatedOutput {
    let original_chars = content.len();
    let original_lines = content.lines().count();

    // First pass: truncate any lines longer than MAX_LINE_LENGTH
    let lines: Vec<String> = content.lines().map(|line| {
        if line.len() > MAX_LINE_LENGTH {
            format!("{}... [line truncated at {} chars]", &line[..MAX_LINE_LENGTH], MAX_LINE_LENGTH)
        } else {
            line.to_string()
        }
    }).collect();

    let processed = lines.join("\n");

    // If still within limit, return as-is
    if processed.len() <= limit {
        return TruncatedOutput {
            content: processed,
            was_truncated: original_chars > content.len(), // Only true if lines were truncated
            original_chars,
            original_lines,
            truncated_chars: 0,
            truncated_lines: 0,
        };
    }

    // Second pass: truncate total content
    // Try to truncate at a line boundary
    let mut truncated_content = String::new();
    let mut current_len = 0;
    let mut kept_lines = 0;

    for line in lines.iter() {
        let line_with_newline = format!("{}\n", line);
        if current_len + line_with_newline.len() > limit {
            break;
        }
        truncated_content.push_str(&line_with_newline);
        current_len += line_with_newline.len();
        kept_lines += 1;
    }

    // Remove trailing newline if present
    if truncated_content.ends_with('\n') {
        truncated_content.pop();
    }

    let truncated_lines = original_lines.saturating_sub(kept_lines);
    let truncated_chars = original_chars.saturating_sub(truncated_content.len());

    TruncatedOutput {
        content: truncated_content,
        was_truncated: true,
        original_chars,
        original_lines,
        truncated_chars,
        truncated_lines,
    }
}

/// Simple truncate content if it exceeds the maximum length (backward compatible)
pub fn maybe_truncate(content: &str) -> String {
    let result = truncate_output(content);
    if result.was_truncated {
        result.format_with_message()
    } else {
        result.content
    }
}

/// Truncate content with custom limit (backward compatible)
pub fn maybe_truncate_with_limit(content: &str, limit: usize) -> String {
    let result = truncate_output_with_limit(content, limit);
    if result.was_truncated {
        result.format_with_message()
    } else {
        result.content
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
        // Create content with many lines to exceed limit
        let lines: Vec<String> = (0..10000).map(|i| format!("Line {}: Some content here", i)).collect();
        let content = lines.join("\n");
        assert!(content.len() > MAX_RESPONSE_LEN);

        let result = maybe_truncate(&content);
        assert!(result.contains(TRUNCATED_MESSAGE));
        assert!(result.contains("lines truncated"));
    }

    #[test]
    fn test_maybe_truncate_with_limit() {
        // Use multi-line content for limit testing
        let lines: Vec<String> = (0..100).map(|i| format!("Line {}", i)).collect();
        let content = lines.join("\n");
        let result = maybe_truncate_with_limit(&content, 50);
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
        let lines: Vec<String> = (0..100).map(|i| format!("Line {}", i)).collect();
        let content = lines.join("\n");
        let result = maybe_truncate_by_tokens(&content, 100); // 100 tokens = ~400 chars
        // Result should be truncated since content is larger
        if content.len() > 400 {
            assert!(result.len() < content.len());
        }
    }

    #[test]
    fn test_truncate_output_statistics() {
        let lines: Vec<String> = (0..1000).map(|i| format!("Line {}: content", i)).collect();
        let content = lines.join("\n");

        let result = truncate_output_with_limit(&content, 500);
        assert!(result.was_truncated);
        assert!(result.truncated_lines > 0);
        assert!(result.original_lines == 1000);
    }

    #[test]
    fn test_long_line_truncation() {
        let long_line = "x".repeat(MAX_LINE_LENGTH + 500);
        let content = format!("Short line\n{}\nAnother short line", long_line);

        let result = truncate_output(&content);
        // The long line should be truncated
        assert!(result.content.contains("[line truncated at"));
        assert!(!result.content.contains(&long_line));
    }

    #[test]
    fn test_truncated_output_format() {
        let lines: Vec<String> = (0..1000).map(|i| format!("Line {}", i)).collect();
        let content = lines.join("\n");

        let result = truncate_output_with_limit(&content, 100);
        let formatted = result.format_with_message();

        assert!(formatted.contains("lines truncated"));
        assert!(formatted.contains(TRUNCATED_MESSAGE));
    }

    #[test]
    fn test_no_truncation_needed() {
        let content = "Short content\nWith a few lines\nNothing special";
        let result = truncate_output(&content);

        assert!(!result.was_truncated);
        assert_eq!(result.content, content);
        assert_eq!(result.truncated_lines, 0);
        assert_eq!(result.truncated_chars, 0);
    }
}

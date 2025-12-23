//! Utility functions for agent operations

/// Check if content contains markdown formatting
pub fn is_markdown_content(content: &str) -> bool {
    // Simple heuristics to detect markdown content
    content.contains("# ") ||           // Headers
    content.contains("## ") ||          // Headers
    content.contains("### ") ||         // Headers
    content.contains("* ") ||           // Lists
    content.contains("- ") ||           // Lists
    content.contains("```") ||          // Code blocks
    content.contains("`") ||            // Inline code
    content.contains("**") ||           // Bold
    content.contains("*") ||            // Italic
    content.contains("[") && content.contains("](") || // Links
    content.contains("> ") ||           // Blockquotes
    content.lines().count() > 3 // Multi-line content is likely markdown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_markdown_content_headers() {
        assert!(is_markdown_content("# Header"));
        assert!(is_markdown_content("## Sub Header"));
        assert!(is_markdown_content("### Sub Sub Header"));
    }

    #[test]
    fn test_is_markdown_content_lists() {
        assert!(is_markdown_content("* Item 1\n* Item 2"));
        assert!(is_markdown_content("- Item 1\n- Item 2"));
    }

    #[test]
    fn test_is_markdown_content_code_blocks() {
        assert!(is_markdown_content("```rust\nfn main() {}\n```"));
        assert!(is_markdown_content("Some `inline code` here"));
    }

    #[test]
    fn test_is_markdown_content_formatting() {
        assert!(is_markdown_content("**bold text**"));
        assert!(is_markdown_content("*italic text*"));
        assert!(is_markdown_content(
            "[link](https://example.com)"
        ));
        assert!(is_markdown_content("> blockquote"));
    }

    #[test]
    fn test_is_markdown_content_multiline() {
        let multiline = "Line 1\nLine 2\nLine 3\nLine 4";
        assert!(is_markdown_content(multiline));
    }

    #[test]
    fn test_is_not_markdown_content() {
        assert!(!is_markdown_content("Simple text"));
        assert!(!is_markdown_content("Just a sentence."));
    }

    #[test]
    fn test_markdown_edge_cases() {
        // Test edge cases for markdown detection
        // Empty string is not markdown
        assert!(!is_markdown_content(""));

        // Single line without markdown
        assert!(!is_markdown_content("Hello"));

        // Contains asterisk but is markdown
        assert!(is_markdown_content("* List item"));
        assert!(is_markdown_content("**bold**"));

        // Multiple lines without markdown triggers multiline heuristic
        let multiline_plain = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        assert!(is_markdown_content(multiline_plain));
    }
}

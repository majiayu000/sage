//! Diagnostic commands (doctor, status, usage)
//!
//! Provides system health checks and usage statistics similar to Claude Code's
//! `/doctor` command.

mod checks;
mod doctor;
mod status;
mod types;
mod usage;
mod usage_cmd;

pub use doctor::doctor;
pub use status::status;
pub use usage_cmd::usage_cmd as usage;

#[cfg(test)]
mod tests {
    use super::types::{CheckResult, CheckStatus, format_number};
    use super::usage::{extract_usage_from_content, extract_usage_from_json};

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
    }

    #[test]
    fn test_check_result_icons() {
        let pass = CheckResult::pass("test", "message");
        let warn = CheckResult::warn("test", "message");
        let fail = CheckResult::fail("test", "message");

        assert_eq!(pass.status, CheckStatus::Pass);
        assert_eq!(warn.status, CheckStatus::Warn);
        assert_eq!(fail.status, CheckStatus::Fail);
    }

    #[test]
    fn test_extract_usage_from_json() {
        let json = serde_json::json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "cache_read_input_tokens": 20
            }
        });

        let usage = extract_usage_from_json(&json).unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.cache_read_tokens, 20);
    }

    #[test]
    fn test_extract_usage_from_content_jsonl() {
        let content = r#"
{"usage": {"prompt_tokens": 100, "completion_tokens": 50}}
{"usage": {"prompt_tokens": 200, "completion_tokens": 100}}
"#;

        let usage = extract_usage_from_content(content).unwrap();
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 150);
    }
}

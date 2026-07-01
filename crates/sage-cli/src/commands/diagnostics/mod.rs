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

use colored::Colorize;
use sage_core::diagnostics::{
    DiagnosticBundleSections, FeedbackBundleOutcome, FeedbackConsent, global_diagnostics,
    write_feedback_bundle,
};
use sage_core::telemetry::global_telemetry;
use sage_core::{Config, SageResult};
use std::path::Path;

pub use doctor::doctor;
pub use status::status;
pub use usage_cmd::usage_cmd as usage;

pub async fn feedback(config_file: &str, output: impl AsRef<Path>, yes: bool) -> SageResult<()> {
    let consent = if yes {
        FeedbackConsent::Granted
    } else {
        FeedbackConsent::Declined
    };
    let sections = feedback_sections(config_file);
    match write_feedback_bundle(consent, sections, output.as_ref())? {
        FeedbackBundleOutcome::Declined => {
            println!(
                "{}",
                "Feedback bundle declined; no diagnostic artifact was written.".yellow()
            );
        }
        FeedbackBundleOutcome::Written { path } => {
            println!(
                "{} {}",
                "Redacted feedback bundle written:".green(),
                path.display()
            );
        }
        FeedbackBundleOutcome::Built => {}
    }
    Ok(())
}

fn feedback_sections(config_file: &str) -> DiagnosticBundleSections {
    let config_status = if std::path::Path::new(config_file).exists() {
        "config file present"
    } else {
        "config file missing"
    };
    let telemetry = global_telemetry().get_summary();
    DiagnosticBundleSections {
        doctor_summary: format!(
            "{}; telemetry_events={} dropped={} capacity={}",
            config_status,
            telemetry.total_events,
            telemetry.dropped_events,
            telemetry.event_capacity
        ),
        config_source_stack: vec![format!("config_file={config_file}")],
        provider_summary: Config::default().default_provider,
        proxy_summary: proxy_summary(),
        sandbox_summary: "sandbox summary unavailable in CLI feedback snapshot".to_string(),
        permission_summary: "permission summary unavailable in CLI feedback snapshot".to_string(),
        recent_events: Some(global_diagnostics().snapshot()),
        audit_summaries: Vec::new(),
    }
}

fn proxy_summary() -> String {
    let mut values = Vec::new();
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"] {
        if std::env::var_os(key).is_some() {
            values.push(format!("{key}=set"));
        }
    }
    if values.is_empty() {
        "proxy environment unset".to_string()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::types::{CheckResult, CheckStatus, format_number};
    use super::usage::{extract_usage_from_content, extract_usage_from_json};
    use super::{feedback, feedback_sections};
    use tempfile::tempdir;

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

        let usage = extract_usage_from_json(&json);
        assert!(usage.is_some());
        if let Some(usage) = usage {
            assert_eq!(usage.prompt_tokens, 100);
            assert_eq!(usage.completion_tokens, 50);
            assert_eq!(usage.cache_read_tokens, 20);
        }
    }

    #[test]
    fn test_extract_usage_from_content_jsonl() {
        let content = r#"
{"usage": {"prompt_tokens": 100, "completion_tokens": 50}}
{"usage": {"prompt_tokens": 200, "completion_tokens": 100}}
"#;

        let usage = extract_usage_from_content(content);
        assert!(usage.is_some());
        if let Some(usage) = usage {
            assert_eq!(usage.prompt_tokens, 300);
            assert_eq!(usage.completion_tokens, 150);
        }
    }

    #[tokio::test]
    async fn feedback_bundle_decline_does_not_write_artifact() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("feedback.json");

        feedback("missing-config.json", &output, false)
            .await
            .unwrap();

        assert!(!output.exists());
    }

    #[test]
    fn feedback_sections_include_diagnostics_snapshot() {
        let sections = feedback_sections("missing-config.json");

        assert!(sections.recent_events.is_some());
        assert!(sections.doctor_summary.contains("telemetry_events="));
    }
}

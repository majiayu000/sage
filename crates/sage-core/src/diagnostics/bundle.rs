//! Opt-in feedback diagnostic bundle generation.

use super::{DiagnosticEventSnapshot, DiagnosticRedactor, RedactionReport};
use crate::error::{SageError, SageResult};
use crate::permissions::{PermissionDecision, PermissionDecisionKind, PermissionProfileSource};
use crate::sandbox::violations::Violation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackConsent {
    Granted,
    Declined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecisionKind {
    Allow,
    Deny,
    Ask,
    Unsupported,
    Warning,
    ProviderError,
}

impl From<PermissionDecisionKind> for AuditDecisionKind {
    fn from(value: PermissionDecisionKind) -> Self {
        match value {
            PermissionDecisionKind::Allow => Self::Allow,
            PermissionDecisionKind::Deny => Self::Deny,
            PermissionDecisionKind::Ask => Self::Ask,
            PermissionDecisionKind::Unsupported => Self::Unsupported,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyAuditSummary {
    pub decision: AuditDecisionKind,
    pub source: Option<PermissionProfileSource>,
    pub matched_rule: Option<String>,
    pub reason: String,
    pub redacted_context: String,
}

impl PolicyAuditSummary {
    pub fn redacted(mut self, redactor: &DiagnosticRedactor) -> Self {
        self.reason = redactor.redact_text(&self.reason).value;
        self.redacted_context = redactor.redact_text(&self.redacted_context).value;
        if let Some(rule) = self.matched_rule.as_mut() {
            *rule = redactor.redact_text(rule).value;
        }
        self
    }
}

pub fn audit_permission_decision(
    decision: &PermissionDecision,
    context: impl AsRef<str>,
) -> PolicyAuditSummary {
    PolicyAuditSummary {
        decision: decision.kind.into(),
        source: decision.matched_rule.as_ref().map(|rule| rule.source),
        matched_rule: decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.clone()),
        reason: decision.reason.clone(),
        redacted_context: DiagnosticRedactor::new()
            .redact_text(&format!(
                "audit_key={} context={}",
                decision.audit_key,
                context.as_ref()
            ))
            .value,
    }
}

pub fn audit_sandbox_violation(violation: &Violation) -> PolicyAuditSummary {
    let context = violation.context.clone().unwrap_or_default();
    PolicyAuditSummary {
        decision: if violation.blocked {
            AuditDecisionKind::Deny
        } else {
            AuditDecisionKind::Warning
        },
        source: None,
        matched_rule: Some(violation.violation_type.as_str().to_string()),
        reason: violation.message.clone(),
        redacted_context: DiagnosticRedactor::new()
            .redact_text(&format!(
                "trigger={} context={}",
                violation.trigger, context
            ))
            .value,
    }
}

pub fn audit_provider_error(provider: &str, raw_error: &str) -> PolicyAuditSummary {
    PolicyAuditSummary {
        decision: AuditDecisionKind::ProviderError,
        source: None,
        matched_rule: Some(provider.to_string()),
        reason: "provider error".to_string(),
        redacted_context: DiagnosticRedactor::new().redact_text(raw_error).value,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticBundleSections {
    pub doctor_summary: String,
    pub config_source_stack: Vec<String>,
    pub provider_summary: String,
    pub proxy_summary: String,
    pub sandbox_summary: String,
    pub permission_summary: String,
    pub recent_events: Option<DiagnosticEventSnapshot>,
    pub audit_summaries: Vec<PolicyAuditSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackBundle {
    pub generated_at: DateTime<Utc>,
    pub sections: DiagnosticBundleSections,
    pub redaction_report: RedactionReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackBundleOutcome {
    Declined,
    Built,
    Written { path: PathBuf },
}

pub fn build_feedback_bundle(
    consent: FeedbackConsent,
    sections: DiagnosticBundleSections,
) -> SageResult<Option<FeedbackBundle>> {
    if matches!(consent, FeedbackConsent::Declined) {
        return Ok(None);
    }

    let redactor = DiagnosticRedactor::new();
    let mut report = RedactionReport::default();
    let redacted_sections = redact_sections(sections, &redactor, &mut report)?;

    Ok(Some(FeedbackBundle {
        generated_at: Utc::now(),
        sections: redacted_sections,
        redaction_report: report,
    }))
}

pub fn write_feedback_bundle(
    consent: FeedbackConsent,
    sections: DiagnosticBundleSections,
    path: impl AsRef<Path>,
) -> SageResult<FeedbackBundleOutcome> {
    let Some(bundle) = build_feedback_bundle(consent, sections)? else {
        return Ok(FeedbackBundleOutcome::Declined);
    };

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            SageError::io_with_path(error.to_string(), parent.display().to_string())
        })?;
    }
    let content = serde_json::to_string_pretty(&bundle)
        .map_err(|error| SageError::json(format!("serialize feedback bundle: {error}")))?;
    std::fs::write(path, content)
        .map_err(|error| SageError::io_with_path(error.to_string(), path.display().to_string()))?;
    Ok(FeedbackBundleOutcome::Written {
        path: path.to_path_buf(),
    })
}

fn redact_sections(
    sections: DiagnosticBundleSections,
    redactor: &DiagnosticRedactor,
    report: &mut RedactionReport,
) -> SageResult<DiagnosticBundleSections> {
    let doctor_summary = redact_string(redactor, report, &sections.doctor_summary);
    let provider_summary = redact_string(redactor, report, &sections.provider_summary);
    let proxy_summary = redact_string(redactor, report, &sections.proxy_summary);
    let sandbox_summary = redact_string(redactor, report, &sections.sandbox_summary);
    let permission_summary = redact_string(redactor, report, &sections.permission_summary);
    let config_source_stack = sections
        .config_source_stack
        .iter()
        .map(|value| redact_string(redactor, report, value))
        .collect();
    let recent_events = sections
        .recent_events
        .map(|snapshot| redact_snapshot(snapshot, redactor, report))
        .transpose()?;
    let audit_summaries = sections
        .audit_summaries
        .into_iter()
        .map(|summary| summary.redacted(redactor))
        .collect();

    Ok(DiagnosticBundleSections {
        doctor_summary,
        config_source_stack,
        provider_summary,
        proxy_summary,
        sandbox_summary,
        permission_summary,
        recent_events,
        audit_summaries,
    })
}

fn redact_string(
    redactor: &DiagnosticRedactor,
    report: &mut RedactionReport,
    value: &str,
) -> String {
    let redacted = redactor.redact_text(value);
    report.merge(redacted.report);
    redacted.value
}

fn redact_snapshot(
    mut snapshot: DiagnosticEventSnapshot,
    redactor: &DiagnosticRedactor,
    report: &mut RedactionReport,
) -> SageResult<DiagnosticEventSnapshot> {
    for event in &mut snapshot.events {
        event.source = redact_string(redactor, report, &event.source);
        event.payload_summary = redact_string(redactor, report, &event.payload_summary);
        if let Some(thread_id) = event.thread_id.as_mut() {
            *thread_id = redact_string(redactor, report, thread_id);
        }
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{
        DiagnosticEvent, DiagnosticEventKind, DiagnosticEventRing, DiagnosticSeverity,
        RedactionClass,
    };
    use crate::permissions::{
        PermissionAction, PermissionDecisionEngine, PermissionDecisionInput, PermissionProfile,
        PermissionProfileSource,
    };
    use crate::sandbox::violations::{Violation, ViolationType};
    use tempfile::tempdir;

    fn sections_with_secret() -> DiagnosticBundleSections {
        let ring = DiagnosticEventRing::new(4);
        ring.record(DiagnosticEvent::new(
            DiagnosticEventKind::Provider,
            "provider",
            DiagnosticSeverity::Warn,
            RedactionClass::Secret,
            "provider failed with api_key=sk-secret /Users/alice/project/.env",
        ));
        DiagnosticBundleSections {
            doctor_summary: "doctor ok".to_string(),
            config_source_stack: vec!["/Users/alice/.sage/settings.json".to_string()],
            provider_summary: "Authorization: Bearer sk-secret-token".to_string(),
            proxy_summary: "proxy unset".to_string(),
            sandbox_summary: "sandbox ok".to_string(),
            permission_summary: "deny Read(/Users/alice/.ssh/id_rsa)".to_string(),
            recent_events: Some(ring.snapshot()),
            audit_summaries: Vec::new(),
        }
    }

    #[test]
    fn diagnostics_redaction_runs_before_bundle_write() {
        let bundle = build_feedback_bundle(FeedbackConsent::Granted, sections_with_secret())
            .unwrap()
            .unwrap();
        let text = serde_json::to_string(&bundle).unwrap();

        assert!(!text.contains("sk-secret"));
        assert!(!text.contains("/Users/alice"));
        assert!(bundle.redaction_report.replacements >= 3);
    }

    #[test]
    fn diagnostics_feedback_decline_does_not_write_artifact() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bundle.json");

        let outcome =
            write_feedback_bundle(FeedbackConsent::Declined, sections_with_secret(), &path)
                .unwrap();

        assert_eq!(outcome, FeedbackBundleOutcome::Declined);
        assert!(!path.exists());
    }

    #[test]
    fn audit_policy_source_includes_rule_source_and_redacted_context() {
        let profile = PermissionProfile::default()
            .with_source(PermissionProfileSource::Managed)
            .add_deny("Bash(curl *)", PermissionProfileSource::Managed);
        let engine = PermissionDecisionEngine::new(profile);
        let decision = engine.decide(
            PermissionDecisionInput::new(
                PermissionAction::Exec,
                "Bash",
                vec!["Bash(curl https://example.test)".to_string()],
            )
            .with_working_directory("/Users/alice/project"),
        );

        let summary = audit_permission_decision(&decision, "token=sk-secret");

        assert_eq!(summary.decision, AuditDecisionKind::Deny);
        assert_eq!(summary.source, Some(PermissionProfileSource::Managed));
        assert!(!summary.redacted_context.contains("sk-secret"));
        assert!(!summary.redacted_context.contains("/Users/alice"));
    }

    #[test]
    fn audit_policy_source_redacts_sandbox_and_provider_context() {
        let violation = Violation::blocked(
            ViolationType::SensitiveFileAccess,
            "sensitive file",
            "cat /Users/alice/.ssh/id_rsa",
        );
        let sandbox = audit_sandbox_violation(&violation);
        let provider = audit_provider_error("openai", "Authorization: Bearer sk-secret-token");

        assert!(!sandbox.redacted_context.contains("/Users/alice"));
        assert!(!provider.redacted_context.contains("sk-secret-token"));
    }
}

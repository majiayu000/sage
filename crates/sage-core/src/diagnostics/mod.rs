//! Diagnostics, feedback bundle, and redaction primitives.

mod bundle;
mod event_ring;
mod redaction;

pub use bundle::{
    AuditDecisionKind, DiagnosticBundleSections, FeedbackBundle, FeedbackBundleOutcome,
    FeedbackConsent, PolicyAuditSummary, audit_permission_decision, audit_provider_error,
    audit_sandbox_violation, audit_summaries_from_events, build_feedback_bundle,
    write_feedback_bundle,
};
pub use event_ring::{
    DiagnosticEvent, DiagnosticEventKind, DiagnosticEventRing, DiagnosticEventSnapshot,
    DiagnosticSeverity, RedactionClass, append_diagnostic_event_to_default_store,
    append_diagnostic_event_to_path, default_diagnostic_event_log_path, global_diagnostics,
    persisted_diagnostics_snapshot, persisted_diagnostics_snapshot_from_path,
};
pub use redaction::{DiagnosticRedactor, RedactedText, RedactionReport};

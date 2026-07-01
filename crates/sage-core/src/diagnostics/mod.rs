//! Diagnostics, feedback bundle, and redaction primitives.

mod bundle;
mod event_ring;
mod redaction;

pub use bundle::{
    AuditDecisionKind, DiagnosticBundleSections, FeedbackBundle, FeedbackBundleOutcome,
    FeedbackConsent, PolicyAuditSummary, audit_permission_decision, audit_provider_error,
    audit_sandbox_violation, build_feedback_bundle, write_feedback_bundle,
};
pub use event_ring::{
    DiagnosticEvent, DiagnosticEventKind, DiagnosticEventRing, DiagnosticEventSnapshot,
    DiagnosticSeverity, RedactionClass, global_diagnostics,
};
pub use redaction::{DiagnosticRedactor, RedactedText, RedactionReport};

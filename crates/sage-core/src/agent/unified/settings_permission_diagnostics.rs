//! Diagnostic recording for settings-backed permission blocks.

use crate::diagnostics::{
    DiagnosticEvent, DiagnosticEventKind, DiagnosticRedactor, DiagnosticSeverity, RedactionClass,
    append_diagnostic_event_to_default_store, global_diagnostics,
};
use crate::tools::types::ToolCall;

pub(super) fn record_blocked_result(tool_call: &ToolCall, message: &str) {
    let redactor = DiagnosticRedactor::new();
    let payload_summary = format!(
        "tool={} decision=deny reason={}",
        tool_call.name,
        redactor.redact_text(message).value
    );
    let event = DiagnosticEvent::new(
        DiagnosticEventKind::Permission,
        diagnostic_source(message),
        DiagnosticSeverity::Warn,
        RedactionClass::Sensitive,
        payload_summary,
    );
    global_diagnostics().record(event.clone());
    if let Err(error) = append_diagnostic_event_to_default_store(&event) {
        tracing::debug!(
            "failed to persist settings permission diagnostic event: {}",
            error
        );
    }
}

fn diagnostic_source(message: &str) -> &'static str {
    let message = message.to_ascii_lowercase();
    if message.contains("managed") {
        "managed_policy"
    } else if message.contains("project") {
        "project_settings_permission"
    } else if message.contains("user") {
        "user_settings_permission"
    } else if message.contains("system") {
        "system_settings_permission"
    } else {
        "local_settings_permission"
    }
}

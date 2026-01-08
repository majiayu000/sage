//! Stderr annotation for violation reporting following Claude Code patterns.
//!
//! Provides XML-formatted violation annotations that can be appended to
//! command stderr output for visibility.

use super::types::Violation;
use super::store::ViolationStore;

/// Format violations as XML for stderr annotation
///
/// Returns a formatted XML block that can be appended to stderr:
/// ```xml
/// <sandbox_violations>
///   <violation type="critical_path_removal" severity="critical" blocked="true">
///     <message>Removal of critical path '/' is blocked</message>
///     <trigger>rm -rf /</trigger>
///   </violation>
/// </sandbox_violations>
/// ```
pub fn format_violations_xml(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return String::new();
    }

    let mut xml = String::from("\n<sandbox_violations>\n");

    for v in violations {
        xml.push_str(&format!(
            "  <violation type=\"{}\" severity=\"{}\" blocked=\"{}\">\n",
            v.violation_type.as_str(),
            v.severity.as_str(),
            v.blocked
        ));
        xml.push_str(&format!(
            "    <message>{}</message>\n",
            escape_xml(&v.message)
        ));
        xml.push_str(&format!(
            "    <trigger>{}</trigger>\n",
            escape_xml(&v.trigger)
        ));
        if let Some(ctx) = &v.context {
            xml.push_str(&format!("    <context>{}</context>\n", escape_xml(ctx)));
        }
        xml.push_str("  </violation>\n");
    }

    xml.push_str("</sandbox_violations>\n");
    xml
}

/// Annotate stderr with violations from a store
///
/// Appends violation information to the provided stderr string.
pub fn annotate_stderr(stderr: &str, store: &ViolationStore) -> String {
    let violations = store.get_all();
    if violations.is_empty() {
        return stderr.to_string();
    }

    let annotation = format_violations_xml(&violations);
    format!("{}{}", stderr, annotation)
}

/// Annotate stderr with specific violations
#[allow(dead_code)]
pub fn annotate_stderr_with_violations(stderr: &str, violations: &[Violation]) -> String {
    if violations.is_empty() {
        return stderr.to_string();
    }

    let annotation = format_violations_xml(violations);
    format!("{}{}", stderr, annotation)
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Format a single violation as a human-readable message
#[allow(dead_code)]
pub fn format_violation_message(violation: &Violation) -> String {
    let status = if violation.blocked {
        "BLOCKED"
    } else {
        "WARNING"
    };

    let mut msg = format!(
        "[{}] {} ({}): {}",
        status,
        violation.violation_type.as_str(),
        violation.severity.as_str(),
        violation.message
    );

    if let Some(ctx) = &violation.context {
        msg.push_str(&format!(" - {}", ctx));
    }

    msg
}

/// Format violations as a summary line
#[allow(dead_code)]
pub fn format_violations_summary(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return "No violations".to_string();
    }

    let blocked = violations.iter().filter(|v| v.blocked).count();
    let warnings = violations.len() - blocked;

    match (blocked, warnings) {
        (0, w) => format!("{} warning(s)", w),
        (b, 0) => format!("{} blocked", b),
        (b, w) => format!("{} blocked, {} warning(s)", b, w),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::violations::types::ViolationType;

    #[test]
    fn test_format_violations_xml_empty() {
        let xml = format_violations_xml(&[]);
        assert!(xml.is_empty());
    }

    #[test]
    fn test_format_violations_xml_single() {
        let violations = vec![Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "Attempted to remove /",
            "rm -rf /",
        )];

        let xml = format_violations_xml(&violations);
        assert!(xml.contains("<sandbox_violations>"));
        assert!(xml.contains("type=\"critical_path_removal\""));
        assert!(xml.contains("severity=\"critical\""));
        assert!(xml.contains("blocked=\"true\""));
        assert!(xml.contains("<message>Attempted to remove /</message>"));
        assert!(xml.contains("<trigger>rm -rf /</trigger>"));
    }

    #[test]
    fn test_format_violations_xml_with_context() {
        let violations = vec![Violation::warning(
            ViolationType::SensitiveFileAccess,
            "Accessing sensitive file",
            "cat ~/.ssh/id_rsa",
        )
        .with_context("SSH private key")];

        let xml = format_violations_xml(&violations);
        assert!(xml.contains("<context>SSH private key</context>"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_annotate_stderr() {
        let store = ViolationStore::new(100);
        store.record(Violation::blocked(
            ViolationType::CommandBlocked,
            "Command not allowed",
            "sudo rm",
        ));

        let annotated = annotate_stderr("command failed", &store);
        assert!(annotated.starts_with("command failed"));
        assert!(annotated.contains("<sandbox_violations>"));
    }

    #[test]
    fn test_format_violation_message() {
        let v = Violation::blocked(ViolationType::CriticalPathRemoval, "Test message", "rm /");
        let msg = format_violation_message(&v);
        assert!(msg.contains("[BLOCKED]"));
        assert!(msg.contains("critical_path_removal"));
        assert!(msg.contains("Test message"));
    }

    #[test]
    fn test_format_violations_summary() {
        let violations = vec![
            Violation::blocked(ViolationType::CriticalPathRemoval, "test", "rm /"),
            Violation::warning(ViolationType::PathAccessDenied, "test", "read /etc"),
            Violation::warning(ViolationType::SensitiveFileAccess, "test", "cat .ssh"),
        ];

        let summary = format_violations_summary(&violations);
        assert_eq!(summary, "1 blocked, 2 warning(s)");
    }

    #[test]
    fn test_format_violations_summary_empty() {
        let summary = format_violations_summary(&[]);
        assert_eq!(summary, "No violations");
    }
}

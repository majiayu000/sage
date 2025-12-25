//! Result formatting for security scanner

use crate::tools::security::scanner::types::ScanResult;

/// Format scan result for display
pub fn format_result(result: &ScanResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Security Scan Results ({:?})\n", result.scan_type));
    output.push_str(&format!("Status: {}\n", result.status));
    output.push_str(&format!("Duration: {:.2}s\n\n", result.duration));

    // Summary
    output.push_str("Summary:\n");
    for (severity, count) in &result.summary {
        output.push_str(&format!("  {}: {}\n", severity, count));
    }
    output.push_str("\n");

    // Detailed findings
    if !result.findings.is_empty() {
        output.push_str("Findings:\n");
        for (i, finding) in result.findings.iter().enumerate() {
            output.push_str(&format!("{}. [{}] {} ({})\n",
                i + 1, finding.severity, finding.title, finding.id));
            output.push_str(&format!("   {}\n", finding.description));

            if let Some(file) = &finding.file {
                if let Some(line) = finding.line {
                    output.push_str(&format!("   Location: {}:{}\n", file, line));
                } else {
                    output.push_str(&format!("   File: {}\n", file));
                }
            }

            if let Some(cwe_id) = &finding.cwe_id {
                output.push_str(&format!("   CWE: {}\n", cwe_id));
            }

            if let Some(cve_id) = &finding.cve_id {
                output.push_str(&format!("   CVE: {}\n", cve_id));
            }

            output.push_str("\n");
        }
    }

    output
}

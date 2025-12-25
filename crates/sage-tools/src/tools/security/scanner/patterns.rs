//! Mock pattern data for security findings
//!
//! In a real implementation, this would integrate with actual security scanners.

use std::collections::HashMap;
use crate::tools::security::scanner::types::{SecurityFinding, ScanType};

/// Generate mock SAST findings
pub fn generate_sast_findings() -> Vec<SecurityFinding> {
    vec![
        SecurityFinding {
            id: "SAST-001".to_string(),
            severity: "HIGH".to_string(),
            title: "SQL Injection Vulnerability".to_string(),
            description: "Potential SQL injection found in database query".to_string(),
            file: Some("src/database.rs".to_string()),
            line: Some(42),
            column: Some(15),
            cwe_id: Some("CWE-89".to_string()),
            cve_id: None,
            confidence: Some("HIGH".to_string()),
        },
        SecurityFinding {
            id: "SAST-002".to_string(),
            severity: "MEDIUM".to_string(),
            title: "Hardcoded Secret".to_string(),
            description: "Hardcoded API key found in source code".to_string(),
            file: Some("src/config.rs".to_string()),
            line: Some(15),
            column: Some(25),
            cwe_id: Some("CWE-798".to_string()),
            cve_id: None,
            confidence: Some("MEDIUM".to_string()),
        },
    ]
}

/// Generate mock dependency findings
pub fn generate_dependency_findings() -> Vec<SecurityFinding> {
    vec![
        SecurityFinding {
            id: "DEP-001".to_string(),
            severity: "CRITICAL".to_string(),
            title: "Vulnerable Dependency".to_string(),
            description: "reqwest 0.10.0 has known security vulnerability".to_string(),
            file: Some("Cargo.toml".to_string()),
            line: Some(20),
            column: None,
            cwe_id: None,
            cve_id: Some("CVE-2023-1234".to_string()),
            confidence: Some("HIGH".to_string()),
        },
    ]
}

/// Generate mock secret findings
pub fn generate_secret_findings() -> Vec<SecurityFinding> {
    vec![
        SecurityFinding {
            id: "SEC-001".to_string(),
            severity: "HIGH".to_string(),
            title: "AWS Access Key".to_string(),
            description: "Potential AWS access key found in file".to_string(),
            file: Some(".env".to_string()),
            line: Some(5),
            column: Some(12),
            cwe_id: Some("CWE-798".to_string()),
            cve_id: None,
            confidence: Some("HIGH".to_string()),
        },
    ]
}

/// Generate mock license findings
pub fn generate_license_findings() -> Vec<SecurityFinding> {
    vec![
        SecurityFinding {
            id: "LIC-001".to_string(),
            severity: "MEDIUM".to_string(),
            title: "GPL License Detected".to_string(),
            description: "GPL-licensed dependency may not be compatible with commercial use".to_string(),
            file: Some("Cargo.toml".to_string()),
            line: Some(25),
            column: None,
            cwe_id: None,
            cve_id: None,
            confidence: Some("HIGH".to_string()),
        },
    ]
}

/// Generate mock full scan findings
pub fn generate_full_scan_findings() -> Vec<SecurityFinding> {
    vec![
        SecurityFinding {
            id: "FULL-001".to_string(),
            severity: "CRITICAL".to_string(),
            title: "Multiple Security Issues".to_string(),
            description: "Found 5 security issues across SAST, dependencies, and secrets".to_string(),
            file: None,
            line: None,
            column: None,
            cwe_id: None,
            cve_id: None,
            confidence: Some("HIGH".to_string()),
        },
    ]
}

/// Get findings by scan type
pub fn get_findings_by_type(scan_type: &ScanType) -> Vec<SecurityFinding> {
    match scan_type {
        ScanType::Sast => generate_sast_findings(),
        ScanType::Dependencies => generate_dependency_findings(),
        ScanType::Secrets => generate_secret_findings(),
        ScanType::Licenses => generate_license_findings(),
        ScanType::Full => generate_full_scan_findings(),
    }
}

/// Count findings by severity
pub fn count_by_severity(findings: &[SecurityFinding]) -> HashMap<String, usize> {
    let mut summary = HashMap::new();
    for finding in findings {
        *summary.entry(finding.severity.clone()).or_insert(0) += 1;
    }
    summary
}

//! Security scanner type definitions

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Security scan types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanType {
    /// Static Application Security Testing
    Sast,
    /// Dependency vulnerability scanning
    Dependencies,
    /// Secret detection
    Secrets,
    /// License compliance
    Licenses,
    /// Full security scan (all types)
    Full,
}

/// Security scanner operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityOperation {
    /// Scan for vulnerabilities
    Scan {
        scan_type: ScanType,
        path: String,
        output_format: Option<String>,
        exclude_paths: Option<Vec<String>>,
    },
    /// Generate security report
    GenerateReport {
        scan_results: String,
        format: String,
    },
    /// Check for specific vulnerability
    CheckVulnerability {
        cve_id: String,
        path: String,
    },
    /// Audit dependencies
    AuditDependencies {
        path: String,
        package_manager: String,
    },
    /// Scan for secrets
    SecretScan {
        path: String,
        patterns: Option<Vec<String>>,
    },
}

/// Security scanner parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScannerParams {
    /// Security operation
    pub operation: SecurityOperation,
    /// Working directory
    pub working_dir: Option<String>,
}

/// Security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// Finding ID
    pub id: String,
    /// Severity level
    pub severity: String,
    /// Title/summary
    pub title: String,
    /// Description
    pub description: String,
    /// File path
    pub file: Option<String>,
    /// Line number
    pub line: Option<usize>,
    /// Column number
    pub column: Option<usize>,
    /// CWE ID
    pub cwe_id: Option<String>,
    /// CVE ID
    pub cve_id: Option<String>,
    /// Confidence level
    pub confidence: Option<String>,
}

/// Security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Scan type
    pub scan_type: ScanType,
    /// Number of findings by severity
    pub summary: HashMap<String, usize>,
    /// Detailed findings
    pub findings: Vec<SecurityFinding>,
    /// Scan duration in seconds
    pub duration: f64,
    /// Scan status
    pub status: String,
}

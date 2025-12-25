//! Security scanner dispatcher
//!
//! This module dispatches security scan operations to appropriate handlers.

use anyhow::Result;

use crate::tools::security::scanner::types::{SecurityOperation, ScanResult};
use crate::tools::security::scanner::operations::{
    perform_scan, audit_dependencies, scan_secrets,
    check_vulnerability, generate_report,
};

/// Execute security scan operation
pub async fn execute_scan(
    operation: SecurityOperation,
    working_dir: Option<&str>,
) -> Result<ScanResult> {
    let start_time = std::time::Instant::now();

    let result = match operation {
        SecurityOperation::Scan { scan_type, path, output_format, exclude_paths } => {
            perform_scan(scan_type, &path, output_format.as_deref(), exclude_paths.as_ref(), working_dir).await?
        }
        SecurityOperation::AuditDependencies { path, package_manager } => {
            audit_dependencies(&path, &package_manager, working_dir).await?
        }
        SecurityOperation::SecretScan { path, patterns } => {
            scan_secrets(&path, patterns.as_ref(), working_dir).await?
        }
        SecurityOperation::CheckVulnerability { cve_id, path } => {
            check_vulnerability(&cve_id, &path, working_dir).await?
        }
        SecurityOperation::GenerateReport { scan_results, format } => {
            generate_report(&scan_results, &format).await?
        }
    };

    Ok(result)
}

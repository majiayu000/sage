//! Entry point detection logic

use std::path::Path;

use super::detector::{LanguageType, ProjectType};
use super::models::EntryPoint;
use super::patterns::{ImportantFile, ImportantFileType};

/// Find entry points from important files
pub fn find_entry_points(
    important_files: &[ImportantFile],
    project: &ProjectType,
) -> Vec<EntryPoint> {
    important_files
        .iter()
        .filter(|f| f.file_type == ImportantFileType::EntryPoint)
        .map(|f| {
            let entry_type = determine_entry_type(&f.path, project);
            EntryPoint {
                path: f.path.clone(),
                entry_type,
                primary: None,
            }
        })
        .collect()
}

/// Determine the entry point type based on file path and project type
pub fn determine_entry_type(path: &Path, project: &ProjectType) -> String {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    match project.primary_language {
        LanguageType::Rust => {
            if file_name == "main.rs" {
                "main".to_string()
            } else if file_name == "lib.rs" {
                "library".to_string()
            } else {
                "binary".to_string()
            }
        }
        LanguageType::TypeScript | LanguageType::JavaScript => {
            if file_name.contains("server") {
                "server".to_string()
            } else if file_name.contains("app") || file_name.contains("App") {
                "app".to_string()
            } else if file_name.contains("index") {
                "index".to_string()
            } else {
                "module".to_string()
            }
        }
        LanguageType::Python => {
            if file_name == "__main__.py" {
                "main".to_string()
            } else if file_name == "__init__.py" {
                "package".to_string()
            } else if file_name == "app.py" {
                "app".to_string()
            } else {
                "module".to_string()
            }
        }
        LanguageType::Go => {
            if path.to_string_lossy().contains("cmd/") {
                "command".to_string()
            } else {
                "main".to_string()
            }
        }
        _ => "entry".to_string(),
    }
}

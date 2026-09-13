//! File path validation

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::path::{Component, Path, PathBuf};

/// Validate file paths
pub fn validate_paths(config: &Config) -> SageResult<()> {
    // Validate working directory
    if let Some(working_dir) = &config.working_directory {
        if !working_dir.exists() {
            return Err(SageError::config(format!(
                "Working directory does not exist: {}",
                working_dir.display()
            )));
        }
        if !working_dir.is_dir() {
            return Err(SageError::config(format!(
                "Working directory is not a directory: {}",
                working_dir.display()
            )));
        }
    }

    // Validate log file path
    if let Some(log_file) = &config.logging.log_file {
        if let Some(parent) = log_file.parent() {
            if !parent.exists() {
                return Err(SageError::config(format!(
                    "Log file directory does not exist: {}",
                    parent.display()
                )));
            }
        }
    }

    // Reject memory storage paths that escape the working directory before
    // FileMemoryStorage can create_dir_all / write outside the project root.
    if let Some(storage_path) = &config.memory.storage_path {
        let working_dir = match &config.working_directory {
            Some(dir) => dir.clone(),
            None => std::env::current_dir().map_err(|error| {
                SageError::config(format!(
                    "failed to resolve current directory while validating memory.storage_path: {error}"
                ))
            })?,
        };
        resolve_within_working_dir(storage_path, &working_dir)?;
    }

    Ok(())
}

/// Resolve `configured` under `working_dir`, rejecting absolute/`..` escapes.
///
/// Relative paths are joined with `working_dir`. Absolute paths are kept as-is
/// only when they still resolve inside `working_dir` after lexical normalization.
pub fn resolve_within_working_dir(configured: &Path, working_dir: &Path) -> SageResult<PathBuf> {
    let root = normalize_lexical(&make_absolute(working_dir)?);
    let joined = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    };
    let resolved = normalize_lexical(&joined);

    if !resolved.starts_with(&root) {
        return Err(SageError::config(format!(
            "memory.storage_path '{}' escapes working directory '{}'",
            configured.display(),
            root.display()
        )));
    }

    Ok(resolved)
}

fn make_absolute(path: &Path) -> SageResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let cwd = std::env::current_dir().map_err(|error| {
        SageError::config(format!(
            "failed to resolve working directory '{}': {error}",
            path.display()
        ))
    })?;
    Ok(cwd.join(path))
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        LoggingConfig, McpConfig, ModelParameters, ToolConfig, TrajectoryConfig,
    };
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn create_test_config() -> Config {
        let mut model_providers = HashMap::new();
        model_providers.insert(
            "anthropic".to_string(),
            ModelParameters {
                model: "claude-3".to_string(),
                api_key: Some("test_key".to_string()),
                max_tokens: Some(4096),
                temperature: Some(0.7),
                top_p: Some(0.9),
                top_k: Some(40),
                parallel_tool_calls: Some(true),
                max_retries: Some(3),
                base_url: Some("https://api.anthropic.com".to_string()),
                api_version: None,
                stop_sequences: None,
            },
        );

        Config {
            default_provider: "anthropic".to_string(),
            max_steps: Some(50),
            total_token_budget: Some(100000),
            model_providers,
            lakeview_config: None,
            enable_lakeview: false,
            working_directory: Some(std::env::temp_dir()),
            tools: ToolConfig {
                tool_settings: std::collections::HashMap::new(),
                max_execution_time: 300,
                allow_parallel_execution: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                log_to_console: true,
                log_to_file: false,
                log_file: None,
            },
            trajectory: TrajectoryConfig::default(),
            mcp: McpConfig::default(),
            memory: crate::config::model::AgentMemoryConfig::default(),
        }
    }

    #[test]
    fn test_validate_paths_nonexistent_working_directory() {
        let mut config = create_test_config();
        config.working_directory = Some(std::path::PathBuf::from(
            "/nonexistent/path/that/does/not/exist",
        ));

        let result = validate_paths(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Working directory does not exist")
        );
    }

    #[test]
    fn test_validate_paths_working_directory_not_a_directory() {
        let mut config = create_test_config();

        // Create a temporary file (not directory)
        let temp_file = std::env::temp_dir().join("test_file.txt");
        std::fs::write(&temp_file, "test").unwrap();
        config.working_directory = Some(temp_file.clone());

        let result = validate_paths(&config);

        // Clean up
        std::fs::remove_file(&temp_file).ok();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Working directory is not a directory")
        );
    }

    #[test]
    fn test_validate_paths_log_file_directory_not_exist() {
        let mut config = create_test_config();
        config.logging.log_file = Some(std::path::PathBuf::from("/nonexistent/dir/log.txt"));

        let result = validate_paths(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Log file directory does not exist")
        );
    }

    #[test]
    fn resolve_within_working_dir_keeps_default_relative_path() {
        let dir = tempdir().unwrap();
        let resolved =
            resolve_within_working_dir(Path::new(".sage/memory/agent-memory.json"), dir.path())
                .unwrap();

        assert_eq!(
            resolved,
            normalize_lexical(&dir.path().join(".sage/memory/agent-memory.json"))
        );
    }

    #[test]
    fn resolve_within_working_dir_rejects_parent_escape() {
        let dir = tempdir().unwrap();
        let error = resolve_within_working_dir(Path::new("../outside.json"), dir.path())
            .unwrap_err()
            .to_string();

        assert!(error.contains("escapes working directory"));
    }

    #[test]
    fn resolve_within_working_dir_rejects_absolute_escape() {
        let dir = tempdir().unwrap();
        let error = resolve_within_working_dir(Path::new("/tmp/evil-memory.json"), dir.path())
            .unwrap_err()
            .to_string();

        assert!(error.contains("escapes working directory"));
    }

    #[test]
    fn resolve_within_working_dir_allows_absolute_under_working_dir() {
        let dir = tempdir().unwrap();
        let inside = dir.path().join("memory.json");
        let resolved = resolve_within_working_dir(&inside, dir.path()).unwrap();

        assert_eq!(resolved, normalize_lexical(&inside));
    }

    #[test]
    fn test_validate_paths_rejects_escaping_memory_storage_path() {
        let dir = tempdir().unwrap();
        let mut config = create_test_config();
        config.working_directory = Some(dir.path().to_path_buf());
        config.memory.storage_path = Some(PathBuf::from("../escape-memory.json"));

        let result = validate_paths(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("escapes working directory")
        );
    }

    #[test]
    fn test_validate_paths_accepts_relative_memory_storage_path() {
        let dir = tempdir().unwrap();
        let mut config = create_test_config();
        config.working_directory = Some(dir.path().to_path_buf());
        config.memory.storage_path = Some(PathBuf::from(".sage/memory/agent-memory.json"));

        assert!(validate_paths(&config).is_ok());
    }
}

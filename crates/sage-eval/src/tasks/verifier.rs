//! Task verification system
//!
//! Verifiers check whether an evaluation task was completed successfully.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// Result of a verification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierResult {
    /// Whether the verification passed
    pub passed: bool,
    /// Human-readable message
    pub message: String,
    /// Detailed output (e.g., test output)
    pub details: Option<String>,
}

impl VerifierResult {
    /// Create a passing result
    pub fn pass(message: impl Into<String>) -> Self {
        Self {
            passed: true,
            message: message.into(),
            details: None,
        }
    }

    /// Create a failing result
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            passed: false,
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the result
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Verifier for checking task completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Verifier {
    /// Run a command and check exit code
    TestCommand {
        /// Command to run
        command: String,
        /// Expected exit code (default: 0)
        #[serde(default)]
        expected_exit_code: i32,
        /// Working directory relative to sandbox root
        #[serde(default)]
        working_dir: Option<String>,
    },

    /// Check if a file exists
    FileExists {
        /// Path relative to sandbox root
        path: String,
    },

    /// Check if a file contains specific content
    FileContains {
        /// Path relative to sandbox root
        path: String,
        /// Content that must be present
        contains: String,
        /// Whether to ignore case
        #[serde(default)]
        ignore_case: bool,
    },

    /// Check file content matches regex
    FileMatches {
        /// Path relative to sandbox root
        path: String,
        /// Regex pattern to match
        pattern: String,
    },

    /// Check multiple files exist
    FilesExist {
        /// Paths relative to sandbox root
        paths: Vec<String>,
    },

    /// Run multiple verifiers, all must pass
    All {
        /// List of verifiers
        verifiers: Vec<Verifier>,
    },

    /// Run multiple verifiers, at least one must pass
    Any {
        /// List of verifiers
        verifiers: Vec<Verifier>,
    },

    /// Custom script verification
    Script {
        /// Script content to run
        script: String,
        /// Interpreter (default: bash)
        #[serde(default = "default_interpreter")]
        interpreter: String,
    },

    /// Python test runner
    PythonTest {
        /// Test file or directory
        #[serde(default)]
        test_path: Option<String>,
        /// Additional pytest args
        #[serde(default)]
        pytest_args: Vec<String>,
    },

    /// Rust test runner
    RustTest {
        /// Package name (for workspace)
        #[serde(default)]
        package: Option<String>,
        /// Test name filter
        #[serde(default)]
        test_filter: Option<String>,
    },

    /// Go test runner
    GoTest {
        /// Package path
        #[serde(default)]
        package: Option<String>,
        /// Test name filter
        #[serde(default)]
        test_filter: Option<String>,
    },
}

fn default_interpreter() -> String {
    "bash".to_string()
}

fn safe_join_under_root(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(relative);

    if rel_path.is_absolute() {
        return Err(format!("Absolute paths are not allowed: {}", relative));
    }

    if rel_path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!("Path traversal is not allowed: {}", relative));
    }

    Ok(root.join(rel_path))
}

impl Verifier {
    /// Run the verifier in the given sandbox directory
    pub async fn verify(&self, sandbox_root: &Path) -> VerifierResult {
        match self {
            Verifier::TestCommand {
                command,
                expected_exit_code,
                working_dir,
            } => {
                let work_dir = match working_dir {
                    Some(dir) => sandbox_root.join(dir),
                    None => sandbox_root.to_path_buf(),
                };

                let result = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&work_dir)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                match result {
                    Ok(output) => {
                        let exit_code = output.status.code().unwrap_or(-1);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let details = format!("stdout:\n{}\nstderr:\n{}", stdout, stderr);

                        if exit_code == *expected_exit_code {
                            VerifierResult::pass(format!(
                                "Command exited with expected code {}",
                                exit_code
                            ))
                            .with_details(details)
                        } else {
                            VerifierResult::fail(format!(
                                "Command exited with code {}, expected {}",
                                exit_code, expected_exit_code
                            ))
                            .with_details(details)
                        }
                    }
                    Err(e) => VerifierResult::fail(format!("Failed to run command: {}", e)),
                }
            }

            Verifier::FileExists { path } => {
                let full_path = match safe_join_under_root(sandbox_root, path) {
                    Ok(path) => path,
                    Err(msg) => return VerifierResult::fail(msg),
                };

                if full_path.exists() {
                    VerifierResult::pass(format!("File exists: {}", path))
                } else {
                    VerifierResult::fail(format!("File not found: {}", path))
                }
            }

            Verifier::FileContains {
                path,
                contains,
                ignore_case,
            } => {
                let full_path = match safe_join_under_root(sandbox_root, path) {
                    Ok(path) => path,
                    Err(msg) => return VerifierResult::fail(msg),
                };

                match tokio::fs::read_to_string(&full_path).await {
                    Ok(content) => {
                        let (content_cmp, contains_cmp) = if *ignore_case {
                            (content.to_lowercase(), contains.to_lowercase())
                        } else {
                            (content.clone(), contains.clone())
                        };

                        if content_cmp.contains(&contains_cmp) {
                            VerifierResult::pass(format!("File {} contains expected content", path))
                        } else {
                            VerifierResult::fail(format!(
                                "File {} does not contain expected content",
                                path
                            ))
                            .with_details(format!("Expected to find: {}", contains))
                        }
                    }
                    Err(e) => VerifierResult::fail(format!("Failed to read file {}: {}", path, e)),
                }
            }

            Verifier::FileMatches { path, pattern } => {
                let full_path = match safe_join_under_root(sandbox_root, path) {
                    Ok(path) => path,
                    Err(msg) => return VerifierResult::fail(msg),
                };

                match tokio::fs::read_to_string(&full_path).await {
                    Ok(content) => match regex::Regex::new(pattern) {
                        Ok(re) => {
                            if re.is_match(&content) {
                                VerifierResult::pass(format!("File {} matches pattern", path))
                            } else {
                                VerifierResult::fail(format!(
                                    "File {} does not match pattern",
                                    path
                                ))
                                .with_details(format!("Pattern: {}", pattern))
                            }
                        }
                        Err(e) => VerifierResult::fail(format!("Invalid regex pattern: {}", e)),
                    },
                    Err(e) => VerifierResult::fail(format!("Failed to read file {}: {}", path, e)),
                }
            }

            Verifier::FilesExist { paths } => {
                let mut missing = Vec::new();
                for path in paths {
                    let full_path = match safe_join_under_root(sandbox_root, path) {
                        Ok(pathbuf) => pathbuf,
                        Err(msg) => return VerifierResult::fail(msg),
                    };
                    if !full_path.exists() {
                        missing.push(path.clone());
                    }
                }

                if missing.is_empty() {
                    VerifierResult::pass(format!("All {} files exist", paths.len()))
                } else {
                    VerifierResult::fail(format!("Missing files: {}", missing.join(", ")))
                }
            }

            Verifier::All { verifiers } => {
                let mut results = Vec::new();
                for v in verifiers {
                    let result = Box::pin(v.verify(sandbox_root)).await;
                    if !result.passed {
                        return VerifierResult::fail("Not all verifiers passed")
                            .with_details(result.message);
                    }
                    results.push(result);
                }
                VerifierResult::pass(format!("All {} verifiers passed", results.len()))
            }

            Verifier::Any { verifiers } => {
                let mut messages = Vec::new();
                for v in verifiers {
                    let result = Box::pin(v.verify(sandbox_root)).await;
                    if result.passed {
                        return VerifierResult::pass("At least one verifier passed")
                            .with_details(result.message);
                    }
                    messages.push(result.message);
                }
                VerifierResult::fail("No verifiers passed").with_details(messages.join("\n"))
            }

            Verifier::Script {
                script,
                interpreter,
            } => {
                let result = Command::new(interpreter)
                    .arg("-c")
                    .arg(script)
                    .current_dir(sandbox_root)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                match result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let details = format!("stdout:\n{}\nstderr:\n{}", stdout, stderr);

                        if output.status.success() {
                            VerifierResult::pass("Script passed").with_details(details)
                        } else {
                            VerifierResult::fail("Script failed").with_details(details)
                        }
                    }
                    Err(e) => VerifierResult::fail(format!("Failed to run script: {}", e)),
                }
            }

            Verifier::PythonTest {
                test_path,
                pytest_args,
            } => {
                let mut cmd = Command::new("python");
                cmd.arg("-m").arg("pytest");

                if let Some(path) = test_path {
                    cmd.arg(path);
                }

                for arg in pytest_args {
                    cmd.arg(arg);
                }

                cmd.arg("-v");
                cmd.current_dir(sandbox_root);
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());

                match cmd.output().await {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let details = format!("{}\n{}", stdout, stderr);

                        if output.status.success() {
                            VerifierResult::pass("Python tests passed").with_details(details)
                        } else {
                            VerifierResult::fail("Python tests failed").with_details(details)
                        }
                    }
                    Err(e) => VerifierResult::fail(format!("Failed to run pytest: {}", e)),
                }
            }

            Verifier::RustTest {
                package,
                test_filter,
            } => {
                let mut cmd = Command::new("cargo");
                cmd.arg("test");

                if let Some(pkg) = package {
                    cmd.arg("-p").arg(pkg);
                }

                if let Some(filter) = test_filter {
                    cmd.arg(filter);
                }

                cmd.arg("--").arg("--nocapture");
                cmd.current_dir(sandbox_root);
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());

                match cmd.output().await {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let details = format!("{}\n{}", stdout, stderr);

                        if output.status.success() {
                            VerifierResult::pass("Rust tests passed").with_details(details)
                        } else {
                            VerifierResult::fail("Rust tests failed").with_details(details)
                        }
                    }
                    Err(e) => VerifierResult::fail(format!("Failed to run cargo test: {}", e)),
                }
            }

            Verifier::GoTest {
                package,
                test_filter,
            } => {
                let mut cmd = Command::new("go");
                cmd.arg("test");
                cmd.arg("-v");

                if let Some(filter) = test_filter {
                    cmd.arg("-run").arg(filter);
                }

                let pkg = package.as_deref().unwrap_or("./...");
                cmd.arg(pkg);

                cmd.current_dir(sandbox_root);
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());

                match cmd.output().await {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let details = format!("{}\n{}", stdout, stderr);

                        if output.status.success() {
                            VerifierResult::pass("Go tests passed").with_details(details)
                        } else {
                            VerifierResult::fail("Go tests failed").with_details(details)
                        }
                    }
                    Err(e) => VerifierResult::fail(format!("Failed to run go test: {}", e)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_exists_verifier() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "hello").await.unwrap();

        let verifier = Verifier::FileExists {
            path: "test.txt".to_string(),
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(result.passed);

        let verifier = Verifier::FileExists {
            path: "nonexistent.txt".to_string(),
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn test_file_contains_verifier() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "Hello World").await.unwrap();

        let verifier = Verifier::FileContains {
            path: "test.txt".to_string(),
            contains: "World".to_string(),
            ignore_case: false,
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(result.passed);

        let verifier = Verifier::FileContains {
            path: "test.txt".to_string(),
            contains: "world".to_string(),
            ignore_case: true,
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_command_verifier() {
        let temp_dir = TempDir::new().unwrap();

        let verifier = Verifier::TestCommand {
            command: "echo hello".to_string(),
            expected_exit_code: 0,
            working_dir: None,
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(result.passed);

        let verifier = Verifier::TestCommand {
            command: "exit 1".to_string(),
            expected_exit_code: 1,
            working_dir: None,
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_all_verifier() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("a.txt"), "a").await.unwrap();
        tokio::fs::write(temp_dir.path().join("b.txt"), "b").await.unwrap();

        let verifier = Verifier::All {
            verifiers: vec![
                Verifier::FileExists {
                    path: "a.txt".to_string(),
                },
                Verifier::FileExists {
                    path: "b.txt".to_string(),
                },
            ],
        };

        let result = verifier.verify(temp_dir.path()).await;
        assert!(result.passed);
    }
}

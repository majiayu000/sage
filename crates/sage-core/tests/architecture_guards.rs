//! Architecture guard tests for sage workspace.
//!
//! These tests scan source files to enforce design-level consistency:
//! - Error handling style (thiserror vs hand-written Display)
//! - No `Result<_, String>` in sage-core
//! - Provider pattern completeness
//! - No RwLock/Mutex wrapping McpClient
//! - File size limits
//! - No bare generic type names for public types
//!
//! Run: `cargo test --package sage-core --test architecture_guards -- --nocapture`

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk `dir` recursively, collecting .rs files that pass `filter`.
fn collect_rs_files(dir: &Path, filter: &dyn Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if !dir.exists() {
        return result;
    }
    for entry in walkdir(dir) {
        if entry.extension().map_or(false, |e| e == "rs") && filter(&entry) {
            result.push(entry);
        }
    }
    result
}

/// Simple recursive directory walk (no external dep).
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

/// Return the workspace root (two levels up from sage-core/tests/).
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // crates/sage-core
    manifest
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("cannot determine workspace root")
        .to_path_buf()
}

fn is_test_file(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("/tests/")
        || s.contains("/test")
        || s.ends_with("_tests.rs")
        || s.ends_with("_test.rs")
}

fn is_example_file(path: &Path) -> bool {
    path.to_string_lossy().contains("/examples/")
}

/// Strip the workspace root prefix for display.
fn rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

// ---------------------------------------------------------------------------
// RS-ERR-01: Error types must use thiserror, no hand-written Display
// ---------------------------------------------------------------------------

#[test]
fn test_error_types_use_thiserror() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    // Files (relative to workspace root) allowed to have hand-written Display for Error.
    // Each entry should be removed once the file is migrated to thiserror.
    let allowlist: HashSet<&str> = [
        "crates/sage-core/src/recovery/circuit_breaker/types.rs", // generic <E>, thiserror hard
        "crates/sage-core/src/recovery/rate_limiter/types.rs",
        "crates/sage-core/src/agent/outcome.rs",
        "crates/sage-core/src/agent/lifecycle/error.rs",
        "crates/sage-core/src/mcp/protocol.rs",
        "crates/sage-core/src/validation/types.rs",
        "crates/sage-tools/src/tools/utils/enhanced_errors/formatters.rs",
    ]
    .into_iter()
    .collect();

    let files = collect_rs_files(&crates_dir, &|p| !is_test_file(p) && !is_example_file(p));
    let mut violations: Vec<(String, usize, String)> = Vec::new();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let relative = rel(file, &root);
        if allowlist.contains(relative.as_str()) {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Detect: impl ... Display for ...Error
            if trimmed.starts_with("impl")
                && trimmed.contains("Display for")
                && trimmed.contains("Error")
            {
                violations.push((relative.clone(), i + 1, trimmed.to_string()));
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from(
            "\n[RS-ERR-01] Hand-written Display for Error types detected.\n\
             Use #[derive(thiserror::Error)] instead.\n\n",
        );
        for (file, line, text) in &violations {
            msg.push_str(&format!("  {}:{} -> {}\n", file, line, text));
        }
        msg.push_str("\nAdd to allowlist in architecture_guards.rs if intentional.\n");
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// RS-ERR-02: No Result<_, String> in sage-core (use SageError/thiserror)
// ---------------------------------------------------------------------------

#[test]
fn test_no_result_string_in_core() {
    let root = workspace_root();
    let core_src = root.join("crates/sage-core/src");

    // Files allowed to use Result<_, String> (legacy, to be migrated).
    let allowlist: HashSet<&str> = [
        // Layer 0 types module cannot depend on error module
        "crates/sage-core/src/types/provider.rs",
    ]
    .into_iter()
    .collect();

    let files = collect_rs_files(&core_src, &|p| !is_test_file(p) && !is_example_file(p));
    let mut violations: Vec<(String, usize, String)> = Vec::new();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let relative = rel(file, &root);
        if allowlist.contains(relative.as_str()) {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("*") {
                continue;
            }
            // Detect: Result<..., String> patterns (error position).
            // We look for `Result<X, String>` but NOT `HashMap<String, String>` etc.
            // Strategy: check for `Result` followed by `, String>` where `, String>`
            // appears after `Result`.
            if let Some(result_pos) = trimmed.find("Result") {
                let after_result = &trimmed[result_pos..];
                if after_result.contains(", String>") || after_result.contains(",String>") {
                    // Exclude SageResult (it's a type alias for Result<T, SageError>)
                    // by checking if it's literally "Result<" not "SageResult<"
                    let before = &trimmed[..result_pos];
                    let is_sage_result = before.ends_with("Sage");
                    if !is_sage_result {
                        violations.push((relative.clone(), i + 1, trimmed.to_string()));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from(
            "\n[RS-ERR-02] Result<_, String> found in sage-core.\n\
             Use SageResult / thiserror error types instead.\n\n",
        );
        for (file, line, text) in &violations {
            msg.push_str(&format!("  {}:{} -> {}\n", file, line, text));
        }
        msg.push_str("\nAdd to allowlist in architecture_guards.rs if intentional.\n");
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// RS-LLM-01: Provider pattern consistency
// ---------------------------------------------------------------------------

#[test]
fn test_provider_pattern_consistency() {
    let root = workspace_root();
    let providers_dir = root.join("crates/sage-core/src/llm/providers");

    // Files to skip (not individual providers)
    let skip_files: HashSet<&str> = [
        "mod.rs",
        "provider_trait.rs",
        "error_utils.rs",
        "request_builder.rs",
    ]
    .into_iter()
    .collect();

    let mut violations: Vec<String> = Vec::new();

    let files = collect_rs_files(&providers_dir, &|p| {
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        !skip_files.contains(name.as_ref())
            && !name.ends_with("_stream.rs")
            && !name.ends_with("_tests.rs")
    });

    // Read ProviderInstance enum to check registration
    let trait_file = providers_dir.join("provider_trait.rs");
    let trait_content = fs::read_to_string(&trait_file).unwrap_or_default();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let name = file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let has_pub_struct = content.contains("pub struct") && content.contains("Provider");
        let has_chat = content.contains("async fn chat(");
        let has_chat_stream = content.contains("async fn chat_stream(");

        if !has_pub_struct {
            violations.push(format!("{name}.rs: missing `pub struct XxxProvider`"));
        }
        if !has_chat {
            violations.push(format!("{name}.rs: missing `async fn chat(`"));
        }
        if !has_chat_stream {
            violations.push(format!("{name}.rs: missing `async fn chat_stream(`"));
        }

        // Check registration in ProviderInstance enum
        // Convert file name to expected enum variant (e.g., openai -> OpenAI, anthropic -> Anthropic)
        if has_pub_struct && !trait_content.is_empty() {
            // Extract the struct name
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub struct") && trimmed.contains("Provider") {
                    let struct_name = trimmed
                        .trim_start_matches("pub struct ")
                        .split_whitespace()
                        .next()
                        .unwrap_or("");
                    if !trait_content.contains(struct_name) {
                        violations.push(format!(
                            "{name}.rs: `{struct_name}` not found in ProviderInstance enum"
                        ));
                    }
                    break;
                }
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from(
            "\n[RS-LLM-01] Provider pattern violations:\n\
             Each provider must have: pub struct XxxProvider + async fn chat( + async fn chat_stream(\n\n",
        );
        for v in &violations {
            msg.push_str(&format!("  {v}\n"));
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// RS-MCP-01: No RwLock/Mutex wrapping McpClient
// ---------------------------------------------------------------------------

#[test]
fn test_no_rwlock_on_mcp_client() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    let files = collect_rs_files(&crates_dir, &|p| !is_test_file(p) && !is_example_file(p));
    let mut violations: Vec<(String, usize, String)> = Vec::new();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let relative = rel(file, &root);
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("Arc<RwLock<McpClient") || trimmed.contains("Arc<Mutex<McpClient") {
                violations.push((relative.clone(), i + 1, trimmed.to_string()));
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from(
            "\n[RS-MCP-01] Arc<RwLock/Mutex<McpClient>> detected.\n\
             McpClient already uses interior mutability; wrapping with RwLock causes deadlocks.\n\n",
        );
        for (file, line, text) in &violations {
            msg.push_str(&format!("  {}:{} -> {}\n", file, line, text));
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// RS-SIZE-01: File size limits (500 lines for non-test files)
// ---------------------------------------------------------------------------

#[test]
fn test_file_size_limits() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    const MAX_LINES: usize = 500;

    // Files allowed to exceed the limit (legacy, to be split).
    let allowlist: HashSet<&str> = [
        "crates/sage-tools/src/tools/extensions/tool_search.rs",
        "crates/sage-core/src/prompts/system_prompt.rs",
        "crates/sage-tools/src/tools/team/team_manager.rs",
        "crates/sage-core/src/error/user_messages.rs",
        "crates/sage-core/src/prompts/template_engine/parser.rs",
        "crates/sage-core/src/config/onboarding/state.rs",
        "crates/sage-core/src/types/tool.rs",
        "crates/sage-core/src/prompts/builder.rs",
        "crates/sage-core/src/config/provider_registry.rs",
    ]
    .into_iter()
    .collect();

    let files = collect_rs_files(&crates_dir, &|p| !is_test_file(p) && !is_example_file(p));
    let mut violations: Vec<(String, usize)> = Vec::new();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let relative = rel(file, &root);
        if allowlist.contains(relative.as_str()) {
            continue;
        }
        let line_count = content.lines().count();
        if line_count > MAX_LINES {
            violations.push((relative, line_count));
        }
    }

    if !violations.is_empty() {
        let mut msg = format!(
            "\n[RS-SIZE-01] Files exceeding {MAX_LINES} lines (split into submodules):\n\n"
        );
        for (file, count) in &violations {
            msg.push_str(&format!("  {} ({} lines)\n", file, count));
        }
        msg.push_str("\nAdd to allowlist in architecture_guards.rs if splitting is deferred.\n");
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// RS-NAME-01: No bare generic type names for public types
// ---------------------------------------------------------------------------

#[test]
fn test_no_bare_generic_type_names() {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    let bare_names: HashSet<&str> = ["Error", "Config", "Status", "Result", "Context"]
        .into_iter()
        .collect();

    // Files allowed to use bare generic names (legacy).
    let allowlist: HashSet<&str> = [
        "crates/sage-core/src/config/config.rs", // `pub struct Config`
    ]
    .into_iter()
    .collect();

    let files = collect_rs_files(&crates_dir, &|p| !is_test_file(p) && !is_example_file(p));
    let mut violations: Vec<(String, usize, String)> = Vec::new();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let relative = rel(file, &root);
        if allowlist.contains(relative.as_str()) {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Only check pub struct/enum declarations
            if !trimmed.starts_with("pub struct") && !trimmed.starts_with("pub enum") {
                continue;
            }
            // Extract type name (e.g., "pub struct Config {" -> "Config")
            let type_name = trimmed
                .trim_start_matches("pub struct ")
                .trim_start_matches("pub enum ")
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or("");
            if bare_names.contains(type_name) {
                violations.push((relative.clone(), i + 1, trimmed.to_string()));
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from(
            "\n[RS-NAME-01] Bare generic type names detected for public types.\n\
             Use a domain prefix: SageError, ToolConfig, AgentStatus, etc.\n\n",
        );
        for (file, line, text) in &violations {
            msg.push_str(&format!("  {}:{} -> {}\n", file, line, text));
        }
        msg.push_str("\nAdd to allowlist in architecture_guards.rs if intentional.\n");
        panic!("{msg}");
    }
}

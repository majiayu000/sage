//! UI Launcher for modern Ink + React interface

use crate::ui_backend::init_ui_backend;
use sage_core::error::{SageError, SageResult};
use std::path::Path;
use std::process::Command;

/// Launch the modern UI (Ink + React) interface
pub async fn launch_modern_ui(
    config_file: &str,
    trajectory_file: Option<&str>,
    working_dir: Option<&str>,
) -> SageResult<()> {
    // Get the path to the UI directory
    let ui_dir = get_ui_directory()?;

    // Check if Node.js is available
    check_nodejs_available()?;

    // Check if UI dependencies are installed
    ensure_ui_dependencies(&ui_dir).await?;

    // Build the UI if needed
    build_ui_if_needed(&ui_dir).await?;

    // Initialize the UI backend for direct Rust calls
    init_ui_backend();
    println!("✅ Initialized Sage UI backend");

    // Launch UI process
    launch_ui_process(&ui_dir, config_file, trajectory_file, working_dir).await?;

    Ok(())
}

/// Get the UI directory path
fn get_ui_directory() -> SageResult<std::path::PathBuf> {
    // Get the current executable path
    let exe_path = std::env::current_exe()
        .map_err(|e| SageError::Other(format!("Failed to get executable path: {}", e)))?;

    // Navigate to the UI directory relative to the executable
    let ui_dir = exe_path
        .parent()
        .ok_or_else(|| SageError::Other("Failed to get parent directory".to_string()))?
        .parent()
        .ok_or_else(|| SageError::Other("Failed to get grandparent directory".to_string()))?
        .parent()
        .ok_or_else(|| SageError::Other("Failed to get great-grandparent directory".to_string()))?
        .join("crates")
        .join("sage-cli")
        .join("ui");

    if !ui_dir.exists() {
        return Err(SageError::Other(format!(
            "UI directory not found: {}. Please ensure the UI components are properly installed.",
            ui_dir.display()
        )));
    }

    Ok(ui_dir)
}

/// Check if Node.js is available
fn check_nodejs_available() -> SageResult<()> {
    let output = Command::new("node")
        .arg("--version")
        .output()
        .map_err(|e| {
            SageError::Other(format!(
                "Node.js not found. Please install Node.js 20+ to use the modern UI: {}",
                e
            ))
        })?;

    if !output.status.success() {
        return Err(SageError::Other(
            "Node.js is not working properly".to_string(),
        ));
    }

    let version = String::from_utf8_lossy(&output.stdout);
    println!("Using Node.js {}", version.trim());

    Ok(())
}

/// Ensure UI dependencies are installed
async fn ensure_ui_dependencies(ui_dir: &Path) -> SageResult<()> {
    let node_modules = ui_dir.join("node_modules");

    if !node_modules.exists() {
        println!("Installing UI dependencies...");

        let output = Command::new("npm")
            .arg("install")
            .current_dir(ui_dir)
            .output()
            .map_err(|e| SageError::Other(format!("Failed to run npm install: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SageError::Other(format!("npm install failed: {}", stderr)));
        }

        println!("UI dependencies installed successfully.");
    }

    Ok(())
}

/// Build the UI if needed
async fn build_ui_if_needed(ui_dir: &Path) -> SageResult<()> {
    let dist_dir = ui_dir.join("dist");
    let src_dir = ui_dir.join("src");

    // Check if we need to build
    let needs_build = if !dist_dir.exists() {
        true
    } else {
        // Check if any source file is newer than the dist directory
        let dist_modified = std::fs::metadata(&dist_dir)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let src_modified = walkdir::WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| std::fs::metadata(e.path()).ok())
            .filter_map(|m| m.modified().ok())
            .max()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        src_modified > dist_modified
    };

    if needs_build {
        println!("Building UI...");

        let output = Command::new("npm")
            .arg("run")
            .arg("build")
            .current_dir(ui_dir)
            .output()
            .map_err(|e| SageError::Other(format!("Failed to run npm run build: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SageError::Other(format!("UI build failed: {}", stderr)));
        }

        println!("UI built successfully.");
    }

    Ok(())
}

/// Launch the UI process
async fn launch_ui_process(
    ui_dir: &Path,
    config_file: &str,
    trajectory_file: Option<&str>,
    working_dir: Option<&str>,
) -> SageResult<()> {
    let mut cmd = Command::new("node");
    cmd.arg("dist/index.js")
        .arg("--config-file")
        .arg(config_file)
        .current_dir(ui_dir);

    if let Some(trajectory) = trajectory_file {
        cmd.arg("--trajectory-file").arg(trajectory);
    }

    if let Some(work_dir) = working_dir {
        cmd.arg("--working-dir").arg(work_dir);
    }

    // Set environment variables for the UI process
    cmd.env("SAGE_UI_MODE", "modern");

    let status = cmd
        .status()
        .map_err(|e| SageError::Other(format!("Failed to launch UI process: {}", e)))?;

    if !status.success() {
        return Err(SageError::Other("UI process exited with error".to_string()));
    }

    Ok(())
}

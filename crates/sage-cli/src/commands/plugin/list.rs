//! Plugin list and validation commands

use sage_core::error::{SageError, SageResult};
use sage_core::plugins::PluginManifest;
use std::path::Path;

/// List installed plugins
pub async fn list_plugins(format: &str, show_all: bool) -> SageResult<()> {
    let plugins_dir = get_plugins_dir()?;

    if !plugins_dir.exists() {
        if format == "json" {
            println!("[]");
        } else {
            println!("No plugins installed.");
            println!("\nTo install a plugin:");
            println!("  sage plugin install <plugin-name>");
            println!("  sage plugin install --path <local-path>");
        }
        return Ok(());
    }

    let mut plugins = Vec::new();

    for entry in std::fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        if let Ok(manifest) = read_manifest(&manifest_path) {
            let enabled = is_plugin_enabled(&manifest.name)?;

            if show_all || enabled {
                plugins.push((manifest, enabled));
            }
        }
    }

    if format == "json" {
        let output: Vec<serde_json::Value> = plugins
            .iter()
            .map(|(m, enabled)| {
                serde_json::json!({
                    "name": m.name,
                    "version": m.version,
                    "description": m.description,
                    "author": m.author,
                    "enabled": enabled
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if plugins.is_empty() {
            println!("No plugins installed.");
            return Ok(());
        }

        println!("Installed Plugins:\n");
        for (manifest, enabled) in &plugins {
            let status = if *enabled { "enabled" } else { "disabled" };
            let desc = manifest
                .description
                .as_deref()
                .unwrap_or("No description");

            println!(
                "  {} v{} [{}]",
                manifest.name, manifest.version, status
            );
            println!("    {}", desc);
            if let Some(author) = &manifest.author {
                println!("    Author: {}", author);
            }
            println!();
        }
    }

    Ok(())
}

/// Validate a plugin manifest
pub async fn validate_plugin(path: &str) -> SageResult<()> {
    let manifest_path = if Path::new(path).is_dir() {
        Path::new(path).join("manifest.json")
    } else {
        Path::new(path).to_path_buf()
    };

    if !manifest_path.exists() {
        return Err(SageError::invalid_input(format!(
            "Manifest not found: {}",
            manifest_path.display()
        )));
    }

    let content = std::fs::read_to_string(&manifest_path)?;

    // Try to parse
    let manifest: Result<PluginManifest, _> = serde_json::from_str(&content);

    match manifest {
        Ok(m) => {
            // Validate
            match m.validate() {
                Ok(()) => {
                    println!("Plugin manifest is valid.\n");
                    println!("Name: {}", m.name);
                    println!("Version: {}", m.version);
                    if let Some(desc) = &m.description {
                        println!("Description: {}", desc);
                    }
                    if let Some(author) = &m.author {
                        println!("Author: {}", author);
                    }
                    if !m.capabilities.is_empty() {
                        println!("Capabilities: {:?}", m.capabilities);
                    }
                    if !m.permissions.is_empty() {
                        println!("Permissions: {:?}", m.permissions);
                    }
                    if !m.dependencies.is_empty() {
                        println!("Dependencies:");
                        for dep in &m.dependencies {
                            let optional = if dep.optional { " (optional)" } else { "" };
                            println!("  - {} {}{}", dep.name, dep.version, optional);
                        }
                    }
                }
                Err(errors) => {
                    println!("Plugin manifest has validation errors:\n");
                    for error in errors {
                        println!("  - {}", error);
                    }
                    return Err(SageError::invalid_input("Manifest validation failed"));
                }
            }
        }
        Err(e) => {
            println!("Failed to parse manifest:\n  {}", e);
            return Err(SageError::invalid_input("Invalid JSON format"));
        }
    }

    Ok(())
}

/// Get plugins directory
fn get_plugins_dir() -> SageResult<std::path::PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;
    Ok(home.join(".sage/plugins"))
}

/// Read and parse plugin manifest
fn read_manifest(path: &Path) -> SageResult<PluginManifest> {
    let content = std::fs::read_to_string(path)?;
    let manifest: PluginManifest = serde_json::from_str(&content)?;
    Ok(manifest)
}

/// Check if a plugin is enabled
fn is_plugin_enabled(name: &str) -> SageResult<bool> {
    let home = dirs::home_dir()
        .ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;
    let disabled_file = home.join(".sage/plugins").join(name).join(".disabled");
    Ok(!disabled_file.exists())
}

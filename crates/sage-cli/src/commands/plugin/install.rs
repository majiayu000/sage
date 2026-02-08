//! Plugin installation commands

use sage_core::error::{SageError, SageResult};
use sage_core::plugins::PluginManifest;
use std::path::Path;

/// Install a plugin
pub async fn install_plugin(plugin: &str, from_path: bool, force: bool) -> SageResult<()> {
    let plugins_dir = get_plugins_dir()?;

    if from_path {
        // Install from local path
        let source_path = Path::new(plugin);
        if !source_path.exists() {
            return Err(SageError::invalid_input(format!(
                "Plugin path not found: {}",
                plugin
            )));
        }

        // Read and validate manifest
        let manifest_path = if source_path.is_dir() {
            source_path.join("manifest.json")
        } else {
            source_path.to_path_buf()
        };

        let manifest = read_manifest(&manifest_path)?;
        let target_dir = plugins_dir.join(&manifest.name);

        if target_dir.exists() && !force {
            return Err(SageError::invalid_input(format!(
                "Plugin '{}' is already installed. Use --force to reinstall.",
                manifest.name
            )));
        }

        // Copy plugin files
        if source_path.is_dir() {
            copy_dir_recursive(source_path, &target_dir)?;
        } else {
            std::fs::create_dir_all(&target_dir)?;
            std::fs::copy(&manifest_path, target_dir.join("manifest.json"))?;
        }

        println!("Installed plugin '{}' v{}", manifest.name, manifest.version);
    } else {
        // Install from marketplace
        println!("Searching marketplace for '{}'...", plugin);

        // TODO: Implement marketplace search and download
        // For now, show placeholder message
        println!("\nMarketplace installation is not yet implemented.");
        println!("To install from a local path, use:");
        println!("  sage plugin install --path <path-to-plugin>");
    }

    Ok(())
}

/// Uninstall a plugin
pub async fn uninstall_plugin(plugin: &str) -> SageResult<()> {
    let plugins_dir = get_plugins_dir()?;
    let plugin_dir = plugins_dir.join(plugin);

    if !plugin_dir.exists() {
        return Err(SageError::invalid_input(format!(
            "Plugin '{}' is not installed",
            plugin
        )));
    }

    std::fs::remove_dir_all(&plugin_dir)?;
    println!("Uninstalled plugin '{}'", plugin);

    Ok(())
}

/// Update a plugin
pub async fn update_plugin(plugin: &str) -> SageResult<()> {
    if plugin == "all" {
        println!("Checking for updates to all plugins...");
        // TODO: Implement update all
        println!("\nPlugin updates are not yet implemented.");
    } else {
        println!("Checking for updates to '{}'...", plugin);
        // TODO: Implement single plugin update
        println!("\nPlugin updates are not yet implemented.");
    }

    Ok(())
}

/// Get plugins directory
fn get_plugins_dir() -> SageResult<std::path::PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;
    let plugins_dir = home.join(".sage/plugins");
    std::fs::create_dir_all(&plugins_dir)?;
    Ok(plugins_dir)
}

/// Read and parse plugin manifest
fn read_manifest(path: &Path) -> SageResult<PluginManifest> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        SageError::invalid_input(format!("Failed to read manifest: {}", e))
    })?;

    let manifest: PluginManifest = serde_json::from_str(&content).map_err(|e| {
        SageError::invalid_input(format!("Invalid manifest format: {}", e))
    })?;

    // Validate manifest
    if let Err(errors) = manifest.validate() {
        return Err(SageError::invalid_input(format!(
            "Invalid manifest:\n  - {}",
            errors.join("\n  - ")
        )));
    }

    Ok(manifest)
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> SageResult<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

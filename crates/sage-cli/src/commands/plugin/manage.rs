//! Plugin enable/disable commands

use sage_core::error::{SageError, SageResult};

/// Enable a plugin
pub async fn enable_plugin(plugin: &str) -> SageResult<()> {
    let plugin_dir = get_plugin_dir(plugin)?;

    if !plugin_dir.exists() {
        return Err(SageError::invalid_input(format!(
            "Plugin '{}' is not installed",
            plugin
        )));
    }

    let disabled_file = plugin_dir.join(".disabled");
    if disabled_file.exists() {
        std::fs::remove_file(&disabled_file)?;
        println!("Enabled plugin '{}'", plugin);
    } else {
        println!("Plugin '{}' is already enabled", plugin);
    }

    Ok(())
}

/// Disable a plugin
pub async fn disable_plugin(plugin: &str) -> SageResult<()> {
    let plugin_dir = get_plugin_dir(plugin)?;

    if !plugin_dir.exists() {
        return Err(SageError::invalid_input(format!(
            "Plugin '{}' is not installed",
            plugin
        )));
    }

    let disabled_file = plugin_dir.join(".disabled");
    if !disabled_file.exists() {
        std::fs::write(&disabled_file, "")?;
        println!("Disabled plugin '{}'", plugin);
    } else {
        println!("Plugin '{}' is already disabled", plugin);
    }

    Ok(())
}

/// Get plugin directory
fn get_plugin_dir(plugin: &str) -> SageResult<std::path::PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;
    Ok(home.join(".sage/plugins").join(plugin))
}

//! Plugin CLI subcommands
//!
//! Provides commands for managing plugins:
//! - sage plugin install <plugin>   - Install a plugin
//! - sage plugin uninstall <plugin> - Uninstall a plugin
//! - sage plugin enable <plugin>    - Enable a plugin
//! - sage plugin disable <plugin>   - Disable a plugin
//! - sage plugin list               - List all plugins
//! - sage plugin update <plugin>    - Update a plugin
//! - sage plugin validate <path>    - Validate a plugin manifest

mod install;
mod list;
mod manage;
mod marketplace;

pub use install::{install_plugin, uninstall_plugin, update_plugin};
pub use list::{list_plugins, validate_plugin};
pub use manage::{enable_plugin, disable_plugin};
pub use marketplace::MarketplaceAction;

use clap::Subcommand;
use sage_core::error::SageResult;

/// Plugin subcommand actions
#[derive(Subcommand, Clone, Debug)]
pub enum PluginAction {
    /// Install a plugin from marketplace or local path
    Install {
        /// Plugin name or path
        plugin: String,
        /// Install from local path instead of marketplace
        #[arg(long)]
        path: bool,
        /// Force reinstall if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        plugin: String,
    },

    /// Enable a disabled plugin
    Enable {
        /// Plugin name
        plugin: String,
    },

    /// Disable a plugin
    Disable {
        /// Plugin name
        plugin: String,
    },

    /// List all installed plugins
    List {
        /// Output format: "text" or "json"
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Show all plugins including disabled
        #[arg(short, long)]
        all: bool,
    },

    /// Update a plugin to the latest version
    Update {
        /// Plugin name (or "all" to update all)
        plugin: String,
    },

    /// Validate a plugin manifest
    Validate {
        /// Path to plugin directory or manifest.json
        path: String,
    },

    /// Manage plugin marketplaces
    Marketplace {
        #[command(subcommand)]
        action: MarketplaceAction,
    },
}

/// Execute plugin subcommand
pub async fn execute(action: PluginAction) -> SageResult<()> {
    match action {
        PluginAction::Install { plugin, path, force } => {
            install_plugin(&plugin, path, force).await
        }
        PluginAction::Uninstall { plugin } => uninstall_plugin(&plugin).await,
        PluginAction::Enable { plugin } => enable_plugin(&plugin).await,
        PluginAction::Disable { plugin } => disable_plugin(&plugin).await,
        PluginAction::List { format, all } => list_plugins(&format, all).await,
        PluginAction::Update { plugin } => update_plugin(&plugin).await,
        PluginAction::Validate { path } => validate_plugin(&path).await,
        PluginAction::Marketplace { action } => marketplace::execute(action).await,
    }
}

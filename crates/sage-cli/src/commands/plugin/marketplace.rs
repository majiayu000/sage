//! Plugin marketplace commands

use clap::Subcommand;
use sage_core::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};

/// Marketplace subcommand actions
#[derive(Subcommand, Clone, Debug)]
pub enum MarketplaceAction {
    /// Add a marketplace source
    Add {
        /// Marketplace name
        name: String,
        /// Marketplace URL
        url: String,
    },

    /// Remove a marketplace source
    Remove {
        /// Marketplace name
        name: String,
    },

    /// List configured marketplaces
    List,

    /// Search for plugins in marketplaces
    Search {
        /// Search query
        query: String,
    },
}

/// Marketplace configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MarketplaceConfig {
    marketplaces: Vec<Marketplace>,
}

/// A marketplace source
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Marketplace {
    name: String,
    url: String,
    enabled: bool,
}

/// Execute marketplace subcommand
pub async fn execute(action: MarketplaceAction) -> SageResult<()> {
    match action {
        MarketplaceAction::Add { name, url } => add_marketplace(&name, &url).await,
        MarketplaceAction::Remove { name } => remove_marketplace(&name).await,
        MarketplaceAction::List => list_marketplaces().await,
        MarketplaceAction::Search { query } => search_marketplace(&query).await,
    }
}

/// Add a marketplace
async fn add_marketplace(name: &str, url: &str) -> SageResult<()> {
    let mut config = load_marketplace_config()?;

    // Check if already exists
    if config.marketplaces.iter().any(|m| m.name == name) {
        return Err(SageError::invalid_input(format!(
            "Marketplace '{}' already exists",
            name
        )));
    }

    config.marketplaces.push(Marketplace {
        name: name.to_string(),
        url: url.to_string(),
        enabled: true,
    });

    save_marketplace_config(&config)?;
    println!("Added marketplace '{}' ({})", name, url);

    Ok(())
}

/// Remove a marketplace
async fn remove_marketplace(name: &str) -> SageResult<()> {
    let mut config = load_marketplace_config()?;

    let initial_len = config.marketplaces.len();
    config.marketplaces.retain(|m| m.name != name);

    if config.marketplaces.len() == initial_len {
        return Err(SageError::invalid_input(format!(
            "Marketplace '{}' not found",
            name
        )));
    }

    save_marketplace_config(&config)?;
    println!("Removed marketplace '{}'", name);

    Ok(())
}

/// List marketplaces
async fn list_marketplaces() -> SageResult<()> {
    let config = load_marketplace_config()?;

    if config.marketplaces.is_empty() {
        println!("No marketplaces configured.");
        println!("\nTo add a marketplace:");
        println!("  sage plugin marketplace add <name> <url>");
        return Ok(());
    }

    println!("Configured Marketplaces:\n");
    for marketplace in &config.marketplaces {
        let status = if marketplace.enabled {
            "enabled"
        } else {
            "disabled"
        };
        println!("  {} [{}]", marketplace.name, status);
        println!("    URL: {}", marketplace.url);
        println!();
    }

    Ok(())
}

/// Search marketplace for plugins
async fn search_marketplace(query: &str) -> SageResult<()> {
    let config = load_marketplace_config()?;

    if config.marketplaces.is_empty() {
        println!("No marketplaces configured.");
        println!("\nTo add a marketplace:");
        println!("  sage plugin marketplace add <name> <url>");
        return Ok(());
    }

    println!("Searching for '{}'...\n", query);

    // TODO: Implement actual marketplace search
    // This would involve:
    // 1. Fetching plugin index from each marketplace
    // 2. Searching for matching plugins
    // 3. Displaying results

    println!("Marketplace search is not yet implemented.");
    println!("This feature will be available in a future release.");

    Ok(())
}

/// Load marketplace configuration
fn load_marketplace_config() -> SageResult<MarketplaceConfig> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(MarketplaceConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)?;
    let config: MarketplaceConfig = serde_json::from_str(&content)?;

    Ok(config)
}

/// Save marketplace configuration
fn save_marketplace_config(config: &MarketplaceConfig) -> SageResult<()> {
    let config_path = get_config_path()?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&config_path, content)?;

    Ok(())
}

/// Get marketplace config path
fn get_config_path() -> SageResult<std::path::PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;
    Ok(home.join(".sage/marketplaces.json"))
}

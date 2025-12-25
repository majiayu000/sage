//! Configuration source loading logic

use super::types::ConfigSource;
use crate::config::args_loader;
use crate::config::env_loader;
use crate::config::file_loader;
use crate::config::model::Config;
use crate::error::SageResult;

/// Load configuration from a specific source
pub(super) fn load_from_source(source: &ConfigSource) -> SageResult<Config> {
    match source {
        ConfigSource::File(path) => {
            tracing::debug!("Loading config from file: {}", path.display());
            let config = file_loader::load_from_file(path)?;
            tracing::debug!("File config provider: {}", config.default_provider);
            Ok(config)
        }
        ConfigSource::Environment => {
            tracing::debug!("Loading config from environment");
            let config = env_loader::load_from_env()?;
            tracing::debug!("Env config provider: {}", config.default_provider);
            Ok(config)
        }
        ConfigSource::CommandLine(args) => {
            tracing::debug!("Loading config from command line");
            args_loader::load_from_args(args)
        }
        ConfigSource::Default => {
            tracing::debug!("Loading default config");
            let config = Config::default();
            tracing::debug!("Default config provider: {}", config.default_provider);
            Ok(config)
        }
    }
}

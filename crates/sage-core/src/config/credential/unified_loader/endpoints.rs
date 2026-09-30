use super::UnifiedConfigLoader;
use crate::config::file_loader;
use crate::config::model::Config;
use crate::error::SageError;

impl UnifiedConfigLoader {
    /// Load endpoints for every supported provider, including aliases that are
    /// absent from the shipped config and providers omitted by the env loader.
    pub(super) fn apply_env_base_urls(config: &mut Config) {
        for provider in super::super::providers::default_providers() {
            if let Ok(base_url) =
                std::env::var(format!("{}_BASE_URL", provider.name.to_uppercase()))
            {
                let params = config
                    .model_providers
                    .entry(provider.name.clone())
                    .or_insert_with(|| {
                        crate::config::provider_defaults::default_parameters_for_provider(
                            &provider.name,
                        )
                        .unwrap_or_default()
                    });
                params.base_url = Some(base_url);
            }
        }
    }

    /// Honor user endpoints even when an explicit/project file is selected.
    /// Other user config fields keep their existing loading precedence.
    pub(super) fn apply_global_base_urls(&self, config: &mut Config) -> Result<(), SageError> {
        let path = self.global_dir.join("config.json");
        if self.global_dir.is_absolute() && path.exists() {
            let user_config = file_loader::load_from_file(&path)?;
            for (provider, params) in user_config.model_providers {
                if let Some(base_url) = params.base_url {
                    let effective_params = config
                        .model_providers
                        .entry(provider.clone())
                        .or_insert_with(|| {
                            crate::config::provider_defaults::default_parameters_for_provider(
                                &provider,
                            )
                            .unwrap_or_default()
                        });
                    effective_params.base_url = Some(base_url);
                }
            }
        }
        Ok(())
    }
}

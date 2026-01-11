//! Tests for ModelParameters

use super::*;
use crate::config::provider::ApiKeySource;

#[test]
fn test_model_parameters_default() {
    let params = ModelParameters::default();
    assert_eq!(params.model, "gpt-4");
    assert_eq!(params.max_tokens, Some(4096));
    assert_eq!(params.temperature, Some(0.7));
    assert_eq!(params.top_p, Some(1.0));
    assert_eq!(params.parallel_tool_calls, Some(true));
    assert_eq!(params.max_retries, Some(3));
}

#[test]
fn test_model_parameters_get_api_key_from_config() {
    let params = ModelParameters {
        api_key: Some("test_key".to_string()),
        ..Default::default()
    };
    assert_eq!(params.get_api_key(), Some("test_key".to_string()));
}

#[test]
fn test_model_parameters_get_api_key_from_env() {
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "env_key");
    }

    let params = ModelParameters {
        api_key: None,
        ..Default::default()
    };
    // Use provider-specific method
    let key_info = params.get_api_key_info_for_provider("openai");
    assert_eq!(key_info.key, Some("env_key".to_string()));
    assert_eq!(key_info.source, ApiKeySource::StandardEnvVar);

    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
    }
}

#[test]
fn test_model_parameters_get_base_url() {
    let params = ModelParameters {
        base_url: Some("https://custom.api".to_string()),
        ..Default::default()
    };
    assert_eq!(params.get_base_url(), "https://custom.api");
}

#[test]
fn test_model_parameters_get_base_url_default() {
    let params = ModelParameters {
        base_url: None,
        ..Default::default()
    };
    assert_eq!(params.get_base_url(), "https://api.openai.com/v1");
}

#[test]
fn test_model_parameters_get_base_url_for_provider() {
    let params = ModelParameters::default();

    assert_eq!(
        params.get_base_url_for_provider("openai"),
        "https://api.openai.com/v1"
    );
    assert_eq!(
        params.get_base_url_for_provider("anthropic"),
        "https://api.anthropic.com"
    );
    assert_eq!(
        params.get_base_url_for_provider("google"),
        "https://generativelanguage.googleapis.com"
    );
    assert_eq!(
        params.get_base_url_for_provider("ollama"),
        "http://localhost:11434"
    );
    assert_eq!(
        params.get_base_url_for_provider("unknown"),
        "http://localhost:8000"
    );
}

#[test]
fn test_model_parameters_validate_success() {
    let params = ModelParameters {
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        top_p: Some(0.9),
        max_tokens: Some(4096),
        ..Default::default()
    };
    assert!(params.validate().is_ok());
}

#[test]
fn test_model_parameters_validate_empty_model() {
    let params = ModelParameters {
        model: "".to_string(),
        ..Default::default()
    };
    assert!(params.validate().is_err());
}

#[test]
fn test_model_parameters_validate_invalid_temperature() {
    let params = ModelParameters {
        model: "gpt-4".to_string(),
        temperature: Some(3.0), // > 2.0
        ..Default::default()
    };
    assert!(params.validate().is_err());
}

#[test]
fn test_model_parameters_validate_invalid_top_p() {
    let params = ModelParameters {
        model: "gpt-4".to_string(),
        top_p: Some(1.5), // > 1.0
        ..Default::default()
    };
    assert!(params.validate().is_err());
}

#[test]
fn test_model_parameters_validate_zero_max_tokens() {
    let params = ModelParameters {
        model: "gpt-4".to_string(),
        max_tokens: Some(0),
        ..Default::default()
    };
    assert!(params.validate().is_err());
}

#[test]
fn test_model_parameters_to_llm_parameters() {
    let params = ModelParameters {
        model: "gpt-4".to_string(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        top_p: Some(0.9),
        top_k: Some(40),
        stop_sequences: Some(vec!["STOP".to_string()]),
        parallel_tool_calls: Some(true),
        ..Default::default()
    };

    let llm_params = params.to_llm_parameters();
    assert_eq!(llm_params.model, "gpt-4");
    assert_eq!(llm_params.max_tokens, Some(4096));
    assert_eq!(llm_params.temperature, Some(0.7));
    assert_eq!(llm_params.top_p, Some(0.9));
    assert_eq!(llm_params.top_k, Some(40));
    assert_eq!(llm_params.stop, Some(vec!["STOP".to_string()]));
    assert_eq!(llm_params.parallel_tool_calls, Some(true));
}

#[test]
fn test_model_parameters_debug() {
    let params = ModelParameters::default();
    let debug_string = format!("{:?}", params);
    assert!(debug_string.contains("ModelParameters"));
}

#[test]
fn test_model_parameters_clone() {
    let params = ModelParameters::default();
    let cloned = params.clone();
    assert_eq!(params.model, cloned.model);
}

#[test]
fn test_model_parameters_merge_partial_override() {
    let mut base = ModelParameters {
        model: "claude-3-sonnet".to_string(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        top_p: Some(0.9),
        ..Default::default()
    };

    let override_params = ModelParameters {
        model: "".to_string(),  // Empty = don't override
        max_tokens: Some(8192), // Override this
        temperature: None,      // None = keep base
        top_p: None,            // None = keep base
        ..Default::default()
    };

    base.merge(override_params);

    // model should be unchanged (empty string doesn't override)
    assert_eq!(base.model, "claude-3-sonnet");
    // max_tokens should be overridden
    assert_eq!(base.max_tokens, Some(8192));
    // temperature should be preserved
    assert_eq!(base.temperature, Some(0.7));
    // top_p should be preserved
    assert_eq!(base.top_p, Some(0.9));
}

#[test]
fn test_model_parameters_merge_api_key() {
    let mut base = ModelParameters {
        api_key: Some("base_key".to_string()),
        ..Default::default()
    };

    // Override with None should preserve base
    let no_key = ModelParameters {
        api_key: None,
        ..Default::default()
    };
    base.merge(no_key);
    assert_eq!(base.api_key, Some("base_key".to_string()));

    // Override with Some should replace
    let new_key = ModelParameters {
        api_key: Some("new_key".to_string()),
        ..Default::default()
    };
    base.merge(new_key);
    assert_eq!(base.api_key, Some("new_key".to_string()));
}

#[test]
fn test_model_parameters_merge_model_name() {
    let mut base = ModelParameters {
        model: "claude-3-sonnet".to_string(),
        ..Default::default()
    };

    // Override with custom model
    let custom = ModelParameters {
        model: "claude-3-opus".to_string(),
        ..Default::default()
    };
    base.merge(custom);
    assert_eq!(base.model, "claude-3-opus");

    // Empty string should not override
    let empty = ModelParameters {
        model: "".to_string(),
        ..Default::default()
    };
    base.merge(empty);
    assert_eq!(base.model, "claude-3-opus"); // Unchanged
}

#[test]
fn test_model_parameters_merge_all_fields() {
    let mut base = ModelParameters::default();

    let override_all = ModelParameters {
        model: "custom-model".to_string(),
        api_key: Some("key".to_string()),
        max_tokens: Some(16384),
        temperature: Some(0.5),
        top_p: Some(0.8),
        top_k: Some(50),
        parallel_tool_calls: Some(false),
        max_retries: Some(5),
        base_url: Some("https://custom.api".to_string()),
        api_version: Some("2024-01".to_string()),
        stop_sequences: Some(vec!["END".to_string()]),
    };

    base.merge(override_all);

    assert_eq!(base.model, "custom-model");
    assert_eq!(base.api_key, Some("key".to_string()));
    assert_eq!(base.max_tokens, Some(16384));
    assert_eq!(base.temperature, Some(0.5));
    assert_eq!(base.top_p, Some(0.8));
    assert_eq!(base.top_k, Some(50));
    assert_eq!(base.parallel_tool_calls, Some(false));
    assert_eq!(base.max_retries, Some(5));
    assert_eq!(base.base_url, Some("https://custom.api".to_string()));
    assert_eq!(base.api_version, Some("2024-01".to_string()));
    assert_eq!(base.stop_sequences, Some(vec!["END".to_string()]));
}

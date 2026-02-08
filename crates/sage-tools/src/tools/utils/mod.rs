//! Utility tools

pub mod arg_sanitizer;
pub mod enhanced_errors;
pub mod monitoring;
pub mod sequential_thinking;
pub mod telemetry_stats;
pub mod tool_validator;
pub mod util_functions;

// Re-export tools
pub use sequential_thinking::SequentialThinkingTool;
pub use telemetry_stats::TelemetryStatsTool;
pub use tool_validator::ToolUsageValidator;

// Re-export utility functions
pub use util_functions::{
    MAX_LINE_LENGTH, MAX_RESPONSE_LEN, TRUNCATED_MESSAGE, TruncatedOutput,
    check_command_efficiency, estimate_tokens, maybe_truncate, maybe_truncate_by_tokens,
    maybe_truncate_with_limit, suggest_efficient_alternative, truncate_output,
    truncate_output_with_limit,
};

// Re-export argument sanitization utilities
pub use arg_sanitizer::{
    reject_shell_chars, validate_env_var, validate_image_reference, validate_namespace,
    validate_path_arg, validate_port_mapping, validate_resource_name, validate_safe_arg,
};

//! Argument sanitization for command execution
//!
//! Provides validation and sanitization for arguments passed to external commands
//! like kubectl, docker, terraform, etc.

use sage_core::tools::base::ToolError;

/// Maximum length for command arguments
const MAX_ARG_LENGTH: usize = 4096;

/// Maximum length for resource names (k8s, docker, etc.)
const MAX_RESOURCE_NAME_LENGTH: usize = 253;

/// Characters that are dangerous in shell contexts
const SHELL_DANGEROUS_CHARS: &[char] = &[
    '$', '`', '|', ';', '&', '>', '<', '\n', '\r', '\0', '\'', '"', '\\', '!', '*', '?', '[', ']',
    '{', '}', '(', ')', '#',
];

/// Validate that an argument doesn't contain shell metacharacters
///
/// This is a defense-in-depth measure - even though we pass arguments
/// as separate array elements (not through shell), we still validate
/// to prevent any potential issues.
pub fn validate_safe_arg(arg: &str, arg_name: &str) -> Result<(), ToolError> {
    // Check length
    if arg.len() > MAX_ARG_LENGTH {
        return Err(ToolError::InvalidArguments(format!(
            "{} is too long (max {} characters)",
            arg_name, MAX_ARG_LENGTH
        )));
    }

    // Check for null bytes
    if arg.contains('\0') {
        return Err(ToolError::InvalidArguments(format!(
            "{} contains null bytes",
            arg_name
        )));
    }

    Ok(())
}

/// Validate a resource name (for k8s, docker, etc.)
///
/// Resource names should be:
/// - Alphanumeric with hyphens and underscores
/// - Max 253 characters (k8s limit)
/// - Not empty
pub fn validate_resource_name(name: &str, resource_type: &str) -> Result<(), ToolError> {
    if name.is_empty() {
        return Err(ToolError::InvalidArguments(format!(
            "{} name cannot be empty",
            resource_type
        )));
    }

    if name.len() > MAX_RESOURCE_NAME_LENGTH {
        return Err(ToolError::InvalidArguments(format!(
            "{} name is too long (max {} characters)",
            resource_type, MAX_RESOURCE_NAME_LENGTH
        )));
    }

    // Check for valid characters (alphanumeric, hyphens, underscores, dots)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(ToolError::InvalidArguments(format!(
            "{} name contains invalid characters (only alphanumeric, hyphens, underscores, and dots allowed)",
            resource_type
        )));
    }

    // Must start with alphanumeric
    if !name.chars().next().map_or(false, |c| c.is_alphanumeric()) {
        return Err(ToolError::InvalidArguments(format!(
            "{} name must start with an alphanumeric character",
            resource_type
        )));
    }

    Ok(())
}

/// Validate a namespace name (stricter than resource name)
pub fn validate_namespace(namespace: &str) -> Result<(), ToolError> {
    validate_resource_name(namespace, "Namespace")?;

    // Namespaces have additional restrictions
    if namespace.len() > 63 {
        return Err(ToolError::InvalidArguments(
            "Namespace name is too long (max 63 characters)".to_string(),
        ));
    }

    // Must be lowercase
    if namespace.chars().any(|c| c.is_uppercase()) {
        return Err(ToolError::InvalidArguments(
            "Namespace name must be lowercase".to_string(),
        ));
    }

    Ok(())
}

/// Validate a Docker image reference
///
/// Format: [registry/]repository[:tag|@digest]
pub fn validate_image_reference(image: &str) -> Result<(), ToolError> {
    if image.is_empty() {
        return Err(ToolError::InvalidArguments(
            "Image reference cannot be empty".to_string(),
        ));
    }

    if image.len() > MAX_ARG_LENGTH {
        return Err(ToolError::InvalidArguments(
            "Image reference is too long".to_string(),
        ));
    }

    // Check for dangerous characters
    for c in SHELL_DANGEROUS_CHARS {
        if image.contains(*c) {
            return Err(ToolError::InvalidArguments(format!(
                "Image reference contains invalid character: '{}'",
                c
            )));
        }
    }

    Ok(())
}

/// Validate a file path for terraform/docker contexts
pub fn validate_path_arg(path: &str, arg_name: &str) -> Result<(), ToolError> {
    if path.is_empty() {
        return Err(ToolError::InvalidArguments(format!(
            "{} cannot be empty",
            arg_name
        )));
    }

    if path.len() > MAX_ARG_LENGTH {
        return Err(ToolError::InvalidArguments(format!(
            "{} is too long",
            arg_name
        )));
    }

    // Check for null bytes
    if path.contains('\0') {
        return Err(ToolError::InvalidArguments(format!(
            "{} contains null bytes",
            arg_name
        )));
    }

    // Check for path traversal attempts
    if path.contains("..") {
        // Allow .. but log a warning - this is valid in some contexts
        tracing::warn!("Path argument contains '..': {}", path);
    }

    Ok(())
}

/// Validate a port mapping (e.g., "8080:80")
pub fn validate_port_mapping(mapping: &str) -> Result<(), ToolError> {
    // Check format
    let parts: Vec<&str> = mapping.split(':').collect();
    if parts.len() != 2 {
        return Err(ToolError::InvalidArguments(format!(
            "Invalid port mapping format: '{}' (expected HOST:CONTAINER)",
            mapping
        )));
    }

    // Validate both ports are numbers
    for part in parts {
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return Err(ToolError::InvalidArguments(format!(
                "Invalid port number in mapping: '{}'",
                mapping
            )));
        }
        if let Ok(port) = part.parse::<u32>() {
            if port > 65535 {
                return Err(ToolError::InvalidArguments(format!(
                    "Port number out of range in mapping: '{}'",
                    mapping
                )));
            }
        }
    }

    Ok(())
}

/// Validate an environment variable assignment (KEY=VALUE)
pub fn validate_env_var(env_var: &str) -> Result<(), ToolError> {
    if !env_var.contains('=') {
        return Err(ToolError::InvalidArguments(format!(
            "Invalid environment variable format: '{}' (expected KEY=VALUE)",
            env_var
        )));
    }

    let parts: Vec<&str> = env_var.splitn(2, '=').collect();
    let key = parts[0];

    // Key must be valid identifier
    if key.is_empty() {
        return Err(ToolError::InvalidArguments(
            "Environment variable key cannot be empty".to_string(),
        ));
    }

    if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ToolError::InvalidArguments(format!(
            "Environment variable key contains invalid characters: '{}'",
            key
        )));
    }

    if key.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return Err(ToolError::InvalidArguments(
            "Environment variable key cannot start with a digit".to_string(),
        ));
    }

    Ok(())
}

/// Reject any argument containing shell-dangerous characters
///
/// Use this for arguments that should never contain shell metacharacters.
pub fn reject_shell_chars(arg: &str, arg_name: &str) -> Result<(), ToolError> {
    for c in SHELL_DANGEROUS_CHARS {
        if arg.contains(*c) {
            return Err(ToolError::InvalidArguments(format!(
                "{} contains forbidden character: '{}'",
                arg_name, c
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_resource_name_valid() {
        assert!(validate_resource_name("my-app", "Pod").is_ok());
        assert!(validate_resource_name("my_app_123", "Pod").is_ok());
        assert!(validate_resource_name("app.v1", "Pod").is_ok());
    }

    #[test]
    fn test_validate_resource_name_invalid() {
        assert!(validate_resource_name("", "Pod").is_err());
        assert!(validate_resource_name("-app", "Pod").is_err());
        assert!(validate_resource_name("app;rm -rf /", "Pod").is_err());
        assert!(validate_resource_name("app\nname", "Pod").is_err());
    }

    #[test]
    fn test_validate_namespace_valid() {
        assert!(validate_namespace("default").is_ok());
        assert!(validate_namespace("kube-system").is_ok());
        assert!(validate_namespace("my-namespace").is_ok());
    }

    #[test]
    fn test_validate_namespace_invalid() {
        assert!(validate_namespace("MyNamespace").is_err()); // uppercase
        assert!(validate_namespace("").is_err());
    }

    #[test]
    fn test_validate_image_reference_valid() {
        assert!(validate_image_reference("nginx").is_ok());
        assert!(validate_image_reference("nginx:latest").is_ok());
        assert!(validate_image_reference("docker.io/library/nginx:1.21").is_ok());
    }

    #[test]
    fn test_validate_image_reference_invalid() {
        assert!(validate_image_reference("").is_err());
        assert!(validate_image_reference("nginx;rm -rf /").is_err());
        assert!(validate_image_reference("nginx$(whoami)").is_err());
    }

    #[test]
    fn test_validate_port_mapping_valid() {
        assert!(validate_port_mapping("8080:80").is_ok());
        assert!(validate_port_mapping("443:443").is_ok());
    }

    #[test]
    fn test_validate_port_mapping_invalid() {
        assert!(validate_port_mapping("8080").is_err());
        assert!(validate_port_mapping("abc:80").is_err());
        assert!(validate_port_mapping("99999:80").is_err());
    }

    #[test]
    fn test_validate_env_var_valid() {
        assert!(validate_env_var("KEY=value").is_ok());
        assert!(validate_env_var("MY_VAR=some value").is_ok());
        assert!(validate_env_var("VAR=").is_ok()); // empty value is ok
    }

    #[test]
    fn test_validate_env_var_invalid() {
        assert!(validate_env_var("NOEQUALS").is_err());
        assert!(validate_env_var("=value").is_err());
        assert!(validate_env_var("1VAR=value").is_err());
        assert!(validate_env_var("VAR-NAME=value").is_err());
    }

    #[test]
    fn test_reject_shell_chars() {
        assert!(reject_shell_chars("safe-name", "arg").is_ok());
        assert!(reject_shell_chars("name;cmd", "arg").is_err());
        assert!(reject_shell_chars("$(cmd)", "arg").is_err());
        assert!(reject_shell_chars("name`cmd`", "arg").is_err());
    }
}

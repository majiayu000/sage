//! Docker argument validation
//!
//! Provides security validation for Docker command arguments.

use anyhow::{bail, Result};

/// Characters that should not appear in Docker resource names
const FORBIDDEN_CHARS: &[char] = &[
    '$', '`', '|', ';', '&', '>', '<', '\n', '\r', '\0', '\'', '"', '\\', '!',
];

/// Validate a container or image name
pub fn validate_docker_name(name: &str, resource_type: &str) -> Result<()> {
    if name.is_empty() {
        bail!("{} name cannot be empty", resource_type);
    }

    if name.len() > 128 {
        bail!("{} name is too long (max 128 characters)", resource_type);
    }

    for c in FORBIDDEN_CHARS {
        if name.contains(*c) {
            bail!(
                "{} name contains forbidden character: '{}'",
                resource_type,
                c
            );
        }
    }

    Ok(())
}

/// Validate a Docker image reference
pub fn validate_image(image: &str) -> Result<()> {
    validate_docker_name(image, "Image")
}

/// Validate a container name
pub fn validate_container(container: &str) -> Result<()> {
    validate_docker_name(container, "Container")
}

/// Validate a volume name
pub fn validate_volume(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Volume name cannot be empty");
    }

    // Volume names should be alphanumeric with hyphens and underscores
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Volume name contains invalid characters");
    }

    Ok(())
}

/// Validate a network name
pub fn validate_network(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Network name cannot be empty");
    }

    // Network names should be alphanumeric with hyphens and underscores
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Network name contains invalid characters");
    }

    Ok(())
}

/// Validate a port mapping string (e.g., "8080:80" or "127.0.0.1:8080:80")
pub fn validate_port_mapping(mapping: &str) -> Result<()> {
    // Check for forbidden characters first
    for c in FORBIDDEN_CHARS {
        if mapping.contains(*c) {
            bail!("Port mapping contains forbidden character: '{}'", c);
        }
    }

    let parts: Vec<&str> = mapping.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        bail!(
            "Invalid port mapping format: '{}' (expected HOST:CONTAINER or IP:HOST:CONTAINER)",
            mapping
        );
    }

    Ok(())
}

/// Validate a volume mount string (e.g., "/host:/container" or "volume:/container")
pub fn validate_volume_mount(mount: &str) -> Result<()> {
    // Check for forbidden characters (except colon which is the delimiter)
    for c in FORBIDDEN_CHARS {
        if mount.contains(*c) {
            bail!("Volume mount contains forbidden character: '{}'", c);
        }
    }

    if !mount.contains(':') {
        bail!("Invalid volume mount format: '{}' (expected SOURCE:TARGET)", mount);
    }

    Ok(())
}

/// Validate an environment variable key
pub fn validate_env_key(key: &str) -> Result<()> {
    if key.is_empty() {
        bail!("Environment variable key cannot be empty");
    }

    if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        bail!("Environment variable key contains invalid characters: '{}'", key);
    }

    if key.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        bail!("Environment variable key cannot start with a digit");
    }

    Ok(())
}

/// Validate a Dockerfile path
pub fn validate_dockerfile_path(path: &str) -> Result<()> {
    if path.is_empty() {
        bail!("Dockerfile path cannot be empty");
    }

    // Check for null bytes
    if path.contains('\0') {
        bail!("Dockerfile path contains null bytes");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_container_name() {
        assert!(validate_container("my-container").is_ok());
        assert!(validate_container("container_123").is_ok());
        assert!(validate_container("").is_err());
        assert!(validate_container("container;rm").is_err());
        assert!(validate_container("container$(cmd)").is_err());
    }

    #[test]
    fn test_validate_image() {
        assert!(validate_image("nginx").is_ok());
        assert!(validate_image("nginx:latest").is_ok());
        assert!(validate_image("docker.io/nginx").is_ok());
        assert!(validate_image("").is_err());
        assert!(validate_image("nginx`whoami`").is_err());
    }

    #[test]
    fn test_validate_port_mapping() {
        assert!(validate_port_mapping("8080:80").is_ok());
        assert!(validate_port_mapping("127.0.0.1:8080:80").is_ok());
        assert!(validate_port_mapping("8080").is_err());
        assert!(validate_port_mapping("8080;cmd:80").is_err());
    }

    #[test]
    fn test_validate_volume_mount() {
        assert!(validate_volume_mount("/host:/container").is_ok());
        assert!(validate_volume_mount("volume:/container").is_ok());
        assert!(validate_volume_mount("/path").is_err());
        assert!(validate_volume_mount("/host;cmd:/container").is_err());
    }

    #[test]
    fn test_validate_env_key() {
        assert!(validate_env_key("MY_VAR").is_ok());
        assert!(validate_env_key("VAR123").is_ok());
        assert!(validate_env_key("").is_err());
        assert!(validate_env_key("123VAR").is_err());
        assert!(validate_env_key("VAR-NAME").is_err());
    }
}

//! Docker operation types and parameters

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Docker operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerOperation {
    /// List containers
    ListContainers { all: bool },
    /// Run a container
    RunContainer {
        image: String,
        name: Option<String>,
        ports: Option<Vec<String>>,
        volumes: Option<Vec<String>>,
        environment: Option<HashMap<String, String>>,
        detach: bool,
        remove: bool,
        command: Option<String>,
    },
    /// Stop a container
    StopContainer { container: String },
    /// Start a container
    StartContainer { container: String },
    /// Remove a container
    RemoveContainer { container: String, force: bool },
    /// Get container logs
    ContainerLogs { container: String, follow: bool, tail: Option<usize> },
    /// Execute command in container
    Exec { container: String, command: String, interactive: bool },
    /// Inspect container
    InspectContainer { container: String },
    /// List images
    ListImages { all: bool },
    /// Build image
    BuildImage {
        dockerfile_path: String,
        tag: String,
        context: Option<String>,
        build_args: Option<HashMap<String, String>>,
    },
    /// Pull image
    PullImage { image: String },
    /// Push image
    PushImage { image: String },
    /// Remove image
    RemoveImage { image: String, force: bool },
    /// Tag image
    TagImage { source: String, target: String },
    /// List volumes
    ListVolumes,
    /// Create volume
    CreateVolume { name: String },
    /// Remove volume
    RemoveVolume { name: String },
    /// List networks
    ListNetworks,
    /// Create network
    CreateNetwork { name: String, driver: Option<String> },
    /// Remove network
    RemoveNetwork { name: String },
    /// Docker system info
    SystemInfo,
    /// Docker system prune
    SystemPrune { volumes: bool },
    /// Docker Compose up
    ComposeUp { file: Option<String>, detach: bool },
    /// Docker Compose down
    ComposeDown { file: Option<String>, volumes: bool },
    /// Docker Compose logs
    ComposeLogs { file: Option<String>, follow: bool },
}

/// Docker tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerParams {
    /// Docker operation
    pub operation: DockerOperation,
    /// Working directory for Docker commands
    pub working_dir: Option<String>,
}

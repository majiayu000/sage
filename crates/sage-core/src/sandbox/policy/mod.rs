//! Sandbox policies for access control

mod command_policy;
mod network_policy;
mod path_policy;

pub use command_policy::CommandPolicy;
pub use network_policy::NetworkPolicy;
pub use path_policy::PathPolicy;

use super::SandboxError;
use super::config::SandboxConfig;

/// Combined sandbox policy
#[derive(Debug)]
pub struct SandboxPolicy {
    pub path_policy: PathPolicy,
    pub command_policy: CommandPolicy,
    pub network_policy: NetworkPolicy,
}

impl SandboxPolicy {
    /// Create policy from configuration
    pub fn from_config(config: &SandboxConfig) -> Result<Self, SandboxError> {
        Ok(Self {
            path_policy: PathPolicy::from_config(config)?,
            command_policy: CommandPolicy::from_config(config)?,
            network_policy: NetworkPolicy::from_config(config)?,
        })
    }
}

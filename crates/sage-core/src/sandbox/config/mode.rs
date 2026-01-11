//! Sandbox mode and validation strictness types.

use serde::{Deserialize, Serialize};

/// Sandbox operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxMode {
    /// Permissive mode - minimal restrictions
    Permissive,
    /// Restricted mode - moderate restrictions
    Restricted,
    /// Strict mode - maximum restrictions
    Strict,
    /// Custom mode - user-defined restrictions
    Custom,
}

impl Default for SandboxMode {
    fn default() -> Self {
        Self::Restricted
    }
}

/// Validation strictness level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ValidationStrictness {
    /// Minimal validation - only block critical issues
    Minimal,
    /// Standard validation - balanced security
    #[default]
    Standard,
    /// Strict validation - maximum security
    Strict,
}

impl ValidationStrictness {
    /// Check if chaining is allowed at this strictness level
    pub fn allows_chaining(&self) -> bool {
        match self {
            ValidationStrictness::Minimal => true,
            ValidationStrictness::Standard => true,
            ValidationStrictness::Strict => false,
        }
    }

    /// Check if background execution is allowed
    pub fn allows_background(&self) -> bool {
        match self {
            ValidationStrictness::Minimal => true,
            ValidationStrictness::Standard => true,
            ValidationStrictness::Strict => false,
        }
    }
}

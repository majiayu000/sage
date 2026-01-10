//! Skill source types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Skill source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSourceInfo {
    /// Source location type
    pub source: SkillSourceType,

    /// Base directory for relative paths in skill
    pub base_dir: Option<PathBuf>,
}

impl Default for SkillSourceInfo {
    fn default() -> Self {
        Self {
            source: SkillSourceType::Builtin,
            base_dir: None,
        }
    }
}

/// Skill source location type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSourceType {
    /// Built-in skill
    Builtin,
    /// Project skill (.sage/skills/)
    Project(PathBuf),
    /// User skill (~/.config/sage/skills/)
    User(PathBuf),
    /// MCP-provided skill
    Mcp(String),
}

impl Default for SkillSourceType {
    fn default() -> Self {
        Self::Builtin
    }
}

/// Type alias for backward compatibility
pub type SkillSource = SkillSourceType;

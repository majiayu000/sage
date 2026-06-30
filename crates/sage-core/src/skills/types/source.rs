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
    /// Extension package-provided skill
    Package {
        /// Package id that owns this skill.
        package_id: String,
        /// Asset id inside the package.
        asset_id: String,
        /// Installed package root.
        package_root: PathBuf,
    },
}

impl Default for SkillSourceType {
    fn default() -> Self {
        Self::Builtin
    }
}

/// Type alias for backward compatibility
pub type SkillSource = SkillSourceType;

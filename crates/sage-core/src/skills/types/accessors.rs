//! Skill accessor methods for backward compatibility

use super::skill::Skill;
use super::source::SkillSourceType;
use std::path::PathBuf;

impl Skill {
    /// Get skill name
    #[inline]
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Get skill description
    #[inline]
    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    /// Get display name
    #[inline]
    pub fn display_name(&self) -> Option<&str> {
        self.metadata.display_name.as_deref()
    }

    /// Get version
    #[inline]
    pub fn version(&self) -> Option<&str> {
        self.metadata.version.as_deref()
    }

    /// Get priority
    #[inline]
    pub fn priority(&self) -> i32 {
        self.invocation.priority
    }

    /// Check if enabled
    #[inline]
    pub fn enabled(&self) -> bool {
        self.invocation.enabled
    }

    /// Set enabled status
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.invocation.enabled = enabled;
    }

    /// Check if model can invoke
    #[inline]
    pub fn model_invocable(&self) -> bool {
        self.invocation.model_invocable
    }

    /// Check if user can invoke
    #[inline]
    pub fn user_invocable(&self) -> bool {
        self.invocation.user_invocable
    }

    /// Get argument hint
    #[inline]
    pub fn argument_hint(&self) -> Option<&str> {
        self.invocation.argument_hint.as_deref()
    }

    /// Get model override
    #[inline]
    pub fn model(&self) -> Option<&str> {
        self.invocation.model.as_deref()
    }

    /// Get source type
    #[inline]
    pub fn source(&self) -> &SkillSourceType {
        &self.source_info.source
    }

    /// Get base directory
    #[inline]
    pub fn base_dir(&self) -> Option<&PathBuf> {
        self.source_info.base_dir.as_ref()
    }
}

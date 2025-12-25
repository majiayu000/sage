//! Skill lookup and matching logic

use super::super::types::{Skill, SkillContext};
use super::types::SkillRegistry;

impl SkillRegistry {
    /// Find matching skills for a context
    pub fn find_matching(&self, context: &SkillContext) -> Vec<&Skill> {
        let mut matching: Vec<_> = self
            .skills
            .values()
            .filter(|s| s.matches(context))
            .collect();

        // Sort by priority (highest first)
        matching.sort_by(|a, b| b.priority.cmp(&a.priority));

        matching
    }

    /// Find the best matching skill for a context
    pub fn find_best_match(&self, context: &SkillContext) -> Option<&Skill> {
        self.find_matching(context).first().copied()
    }
}

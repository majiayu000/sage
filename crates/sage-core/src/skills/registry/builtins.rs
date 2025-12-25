//! Built-in skill registration

use super::super::types::{Skill, SkillSource, SkillTrigger, TaskType};
use super::types::SkillRegistry;

impl SkillRegistry {
    /// Register built-in skills
    pub fn register_builtins(&mut self) {
        // Rust Expert skill
        self.register(
            Skill::new("rust-expert", "Expert Rust programming assistance")
                .with_prompt(include_str!("../builtin_prompts/rust_expert.txt"))
                .with_trigger(SkillTrigger::FileExtension("rs".to_string()))
                .with_trigger(SkillTrigger::Keyword("rust".to_string()))
                .with_trigger(SkillTrigger::Keyword("cargo".to_string()))
                .with_priority(10)
                .with_source(SkillSource::Builtin),
        );

        // Testing skill
        self.register(
            Skill::new(
                "comprehensive-testing",
                "Test-driven development and testing best practices",
            )
            .with_prompt(include_str!("../builtin_prompts/testing.txt"))
            .with_trigger(SkillTrigger::TaskType(TaskType::Testing))
            .with_trigger(SkillTrigger::Keyword("test".to_string()))
            .with_trigger(SkillTrigger::Keyword("spec".to_string()))
            .with_priority(8)
            .with_source(SkillSource::Builtin),
        );

        // Debugging skill
        self.register(
            Skill::new("systematic-debugging", "Systematic debugging methodology")
                .with_prompt(include_str!("../builtin_prompts/debugging.txt"))
                .with_trigger(SkillTrigger::TaskType(TaskType::Debugging))
                .with_trigger(SkillTrigger::Keyword("bug".to_string()))
                .with_trigger(SkillTrigger::Keyword("fix".to_string()))
                .with_trigger(SkillTrigger::Keyword("error".to_string()))
                .with_priority(8)
                .with_source(SkillSource::Builtin),
        );

        // Code review skill
        self.register(
            Skill::new("code-review", "Thorough code review methodology")
                .with_prompt(include_str!("../builtin_prompts/code_review.txt"))
                .with_trigger(SkillTrigger::TaskType(TaskType::Review))
                .with_trigger(SkillTrigger::Keyword("review".to_string()))
                .with_trigger(SkillTrigger::Keyword("pr".to_string()))
                .with_priority(7)
                .with_source(SkillSource::Builtin),
        );

        // Architecture skill
        self.register(
            Skill::new("architecture", "Software architecture and design patterns")
                .with_prompt(include_str!("../builtin_prompts/architecture.txt"))
                .with_trigger(SkillTrigger::TaskType(TaskType::Architecture))
                .with_trigger(SkillTrigger::Keyword("architect".to_string()))
                .with_trigger(SkillTrigger::Keyword("design".to_string()))
                .with_trigger(SkillTrigger::Keyword("pattern".to_string()))
                .with_priority(7)
                .with_source(SkillSource::Builtin),
        );

        // Security skill
        self.register(
            Skill::new(
                "security-analysis",
                "Security analysis and vulnerability detection",
            )
            .with_prompt(include_str!("../builtin_prompts/security.txt"))
            .with_trigger(SkillTrigger::TaskType(TaskType::Security))
            .with_trigger(SkillTrigger::Keyword("security".to_string()))
            .with_trigger(SkillTrigger::Keyword("vulnerability".to_string()))
            .with_trigger(SkillTrigger::Keyword("cve".to_string()))
            .with_priority(9)
            .with_source(SkillSource::Builtin),
        );

        // Git/Commit skill
        self.register(
            Skill::new("git-commit", "Git commit message best practices")
                .with_prompt(include_str!("../builtin_prompts/git_commit.txt"))
                .with_trigger(SkillTrigger::Keyword("commit".to_string()))
                .with_trigger(SkillTrigger::ToolUsage("Bash".to_string()))
                .with_priority(5)
                .with_source(SkillSource::Builtin),
        );
    }
}

//! Skill prompt expansion methods

use super::context::SkillContext;
use super::skill::Skill;
use super::source::SkillSourceType;

impl Skill {
    /// Get the user-facing name (display_name or name)
    pub fn user_facing_name(&self) -> &str {
        self.metadata.user_facing_name()
    }

    /// Check if this skill can be auto-invoked by AI
    pub fn is_auto_invocable(&self) -> bool {
        self.invocation.enabled
            && self.invocation.model_invocable
            && (self.when_to_use.is_some() || !self.triggers.is_empty())
    }

    /// Check if the skill matches a context
    pub fn matches(&self, context: &SkillContext) -> bool {
        if !self.invocation.enabled {
            return false;
        }
        self.triggers.iter().any(|trigger| trigger.matches(context))
    }

    /// Get the skill's full prompt including context
    pub fn get_full_prompt(&self, context: &SkillContext) -> String {
        self.get_prompt_with_args(context, None)
    }

    /// Get the skill's prompt with arguments (Claude Code compatible)
    ///
    /// Replaces `$ARGUMENTS` with the provided args. If `$ARGUMENTS` is not
    /// found in the prompt, appends the args at the end.
    pub fn get_prompt_with_args(&self, context: &SkillContext, args: Option<&str>) -> String {
        let mut prompt = self.prompt.clone();

        // Add base directory context if set
        if let Some(ref base_dir) = self.source_info.base_dir {
            prompt = format!(
                "Base directory for this skill: {}\n\n{}",
                base_dir.display(),
                prompt
            );
        }

        // Replace context variables
        prompt = prompt.replace("$USER_MESSAGE", &context.user_message);
        prompt = prompt.replace("$WORKING_DIR", &context.working_dir.to_string_lossy());

        if let Some(ref file_context) = context.file_context {
            prompt = prompt.replace("$FILE_CONTEXT", file_context);
        }

        // Handle $ARGUMENTS substitution (Claude Code compatible)
        if let Some(args) = args {
            if prompt.contains("$ARGUMENTS") {
                prompt = prompt.replace("$ARGUMENTS", args);
            } else if !args.is_empty() {
                // Append arguments if $ARGUMENTS not found
                prompt = format!("{}\n\nARGUMENTS: {}", prompt, args);
            }
        }

        prompt
    }

    /// Generate XML representation for system prompt injection
    pub fn to_xml(&self) -> String {
        let description = if let Some(ref when) = self.when_to_use {
            format!("{} - {}", self.metadata.description, when)
        } else {
            self.metadata.description.clone()
        };

        let location = match &self.source_info.source {
            SkillSourceType::Project(_) => "project",
            SkillSourceType::User(_) => "user",
            SkillSourceType::Mcp(_) => "mcp",
            SkillSourceType::Builtin => "builtin",
        };

        format!(
            "<skill>\n<name>\n{}\n</name>\n<description>\n{}\n</description>\n<location>\n{}\n</location>\n</skill>",
            self.metadata.name, description, location
        )
    }
}

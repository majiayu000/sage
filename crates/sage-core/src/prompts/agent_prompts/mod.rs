//! Agent-specific prompts
//!
//! Contains specialized prompts for different types of sub-agents.

/// Prompts for sub-agent types
pub struct AgentPrompts;

impl AgentPrompts {
    /// Explore agent prompt - for codebase exploration
    pub const EXPLORE: &'static str = r#"You are an Explore agent specialized in investigating codebases.

Your capabilities:
- Find files by patterns (Glob)
- Search code for keywords (Grep)
- Read and analyze files
- Understand code structure

Focus on:
- Quick, targeted searches
- Finding relevant code efficiently
- Providing concise summaries
- Identifying key files and patterns

Do NOT:
- Write or modify files
- Make implementation decisions
- Spend too much time on exploration"#;

    /// Plan agent prompt - for designing implementations
    pub const PLAN: &'static str = r#"You are a Plan agent specialized in designing implementation approaches.

Your capabilities:
- Analyze requirements
- Design architectures
- Identify trade-offs
- Create step-by-step plans

Focus on:
- Practical, implementable designs
- Considering existing codebase patterns
- Identifying potential issues early
- Clear, actionable steps

Do NOT:
- Write actual code
- Make final decisions without user input
- Over-engineer solutions"#;

    /// Code review agent prompt
    pub const CODE_REVIEW: &'static str = r#"You are a Code Review agent specialized in reviewing code changes.

Your capabilities:
- Analyze code quality
- Identify bugs and issues
- Suggest improvements
- Check for security concerns

Focus on:
- Practical, actionable feedback
- Important issues over style nits
- Security and performance
- Maintainability

Provide feedback in this format:
- [CRITICAL] Must fix before merge
- [SUGGESTION] Nice to have improvements
- [QUESTION] Clarifications needed"#;

    /// General purpose agent prompt
    pub const GENERAL_PURPOSE: &'static str = r#"You are a general-purpose agent for complex, multi-step tasks.

Your capabilities:
- Full access to all tools
- Can read, write, and execute code
- Can search and analyze codebases
- Can run tests and commands

Remember:
- Complete tasks fully
- Prefer code over documentation
- Use tools efficiently
- Ask for clarification when needed"#;

    /// Get agent prompt by type
    pub fn for_agent_type(agent_type: &str) -> Option<&'static str> {
        match agent_type.to_lowercase().as_str() {
            "explore" => Some(Self::EXPLORE),
            "plan" => Some(Self::PLAN),
            "code_review" | "code-review" | "codereview" => Some(Self::CODE_REVIEW),
            "general" | "general_purpose" | "general-purpose" => Some(Self::GENERAL_PURPOSE),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_prompts_exist() {
        assert!(!AgentPrompts::EXPLORE.is_empty());
        assert!(!AgentPrompts::PLAN.is_empty());
        assert!(!AgentPrompts::CODE_REVIEW.is_empty());
        assert!(!AgentPrompts::GENERAL_PURPOSE.is_empty());
    }

    #[test]
    fn test_for_agent_type() {
        assert!(AgentPrompts::for_agent_type("explore").is_some());
        assert!(AgentPrompts::for_agent_type("Explore").is_some());
        assert!(AgentPrompts::for_agent_type("code-review").is_some());
        assert!(AgentPrompts::for_agent_type("unknown").is_none());
    }

    #[test]
    fn test_explore_agent_restrictions() {
        let prompt = AgentPrompts::EXPLORE;
        assert!(prompt.contains("Do NOT"));
        assert!(prompt.contains("Write or modify"));
    }
}

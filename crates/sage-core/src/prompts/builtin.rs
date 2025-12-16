//! Builtin prompt templates and constants
//!
//! Common prompt patterns and templates for various tasks.

/// Collection of builtin prompt templates
pub struct BuiltinPrompts;

impl BuiltinPrompts {
    /// Agent system prompt template
    pub const AGENT_SYSTEM: &'static str = r#"You are {{agent_name}}, an AI assistant specialized in {{specialization}}.

Your capabilities include:
{{capabilities}}

Guidelines:
- Be concise and accurate
- Ask for clarification when needed
- Provide step-by-step explanations for complex tasks
- Cite sources when available

{{additional_instructions}}"#;

    /// Code generation prompt
    pub const CODE_GENERATION: &'static str = r#"Generate {{language}} code that:
{{requirements}}

Requirements:
- Follow best practices for {{language}}
- Include appropriate error handling
- Add comments for complex logic
- {{style_requirements}}

{{additional_context}}"#;

    /// Code review prompt
    pub const CODE_REVIEW: &'static str = r#"Review the following {{language}} code:

```{{language}}
{{code}}
```

Please analyze for:
1. **Code Quality**: Readability, maintainability, and adherence to best practices
2. **Bugs & Issues**: Potential bugs, edge cases, and logical errors
3. **Performance**: Inefficiencies and optimization opportunities
4. **Security**: Potential vulnerabilities and security concerns
5. **Suggestions**: Specific improvements with code examples

{{focus_areas}}"#;

    /// Debugging prompt
    pub const DEBUG_ERROR: &'static str = r#"I'm encountering an error:

**Error Message:**
```
{{error_message}}
```

**Code:**
```{{language}}
{{code}}
```

**Context:**
{{context}}

Please help me:
1. Understand what's causing this error
2. Identify the root cause
3. Provide a fix with explanation"#;

    /// Refactoring prompt
    pub const REFACTOR: &'static str = r#"Refactor the following code to improve {{goal}}:

```{{language}}
{{code}}
```

Requirements:
- Maintain existing functionality
- {{specific_requirements}}
- Explain the changes made

{{constraints}}"#;

    /// Test generation prompt
    pub const GENERATE_TESTS: &'static str = r#"Generate {{test_framework}} tests for the following code:

```{{language}}
{{code}}
```

Requirements:
- Cover happy path and edge cases
- Include error scenarios
- Use descriptive test names
- {{additional_requirements}}"#;

    /// Documentation prompt
    pub const GENERATE_DOCS: &'static str = r#"Generate documentation for the following {{language}} code:

```{{language}}
{{code}}
```

Include:
- Function/class descriptions
- Parameter documentation
- Return value documentation
- Usage examples
- {{doc_style}}"#;

    /// Explanation prompt
    pub const EXPLAIN_CODE: &'static str = r#"Explain the following {{language}} code:

```{{language}}
{{code}}
```

Please explain:
1. What the code does at a high level
2. How it works step by step
3. Key concepts and patterns used
4. Any notable design decisions

Target audience: {{audience}}"#;

    /// Git commit message prompt
    pub const GIT_COMMIT: &'static str = r#"Generate a git commit message for the following changes:

{{diff}}

Requirements:
- Use conventional commit format (type: description)
- First line max 50 characters
- Include body if changes are complex
- {{style_preference}}"#;

    /// PR description prompt
    pub const PR_DESCRIPTION: &'static str = r#"Generate a pull request description for:

**Title:** {{title}}

**Changes:**
{{changes}}

**Related Issues:** {{issues}}

Include:
- Summary of changes
- Motivation and context
- Testing done
- Screenshots if applicable"#;

    /// Architecture review prompt
    pub const ARCHITECTURE_REVIEW: &'static str = r#"Review the following architecture/design:

{{description}}

**Components:**
{{components}}

Please analyze:
1. Overall design quality
2. Separation of concerns
3. Scalability considerations
4. Potential issues or anti-patterns
5. Suggestions for improvement"#;

    /// SQL generation prompt
    pub const SQL_GENERATION: &'static str = r#"Generate a SQL query for:

**Task:** {{task}}

**Schema:**
```sql
{{schema}}
```

**Requirements:**
- Optimize for performance
- Handle NULL values appropriately
- {{dialect}} syntax
- {{additional_constraints}}"#;

    /// API design prompt
    pub const API_DESIGN: &'static str = r#"Design a REST API for:

**Resource:** {{resource}}

**Requirements:**
{{requirements}}

Include:
- Endpoint definitions
- Request/response formats
- Error handling
- Authentication considerations"#;

    /// Conversation summary prompt
    pub const SUMMARIZE_CONVERSATION: &'static str = r#"Summarize the following conversation:

{{conversation}}

Focus on:
- Key decisions made
- Action items
- Important information
- Unresolved questions

Length: {{length}}"#;
}

/// Common prompt fragments for composition
pub struct PromptFragments;

impl PromptFragments {
    /// Be concise instruction
    pub const CONCISE: &'static str = "Be concise and direct. Avoid unnecessary verbosity.";

    /// Step by step instruction
    pub const STEP_BY_STEP: &'static str = "Explain your reasoning step by step.";

    /// Format as JSON instruction
    pub const JSON_FORMAT: &'static str = "Format your response as valid JSON.";

    /// Format as markdown instruction
    pub const MARKDOWN_FORMAT: &'static str = "Format your response using markdown.";

    /// Code only instruction
    pub const CODE_ONLY: &'static str = "Respond with code only, no explanations.";

    /// Include examples instruction
    pub const WITH_EXAMPLES: &'static str = "Include practical examples to illustrate your points.";

    /// Consider edge cases instruction
    pub const EDGE_CASES: &'static str = "Consider edge cases and error scenarios.";

    /// Security focus instruction
    pub const SECURITY_FOCUS: &'static str = "Pay special attention to security implications.";

    /// Performance focus instruction
    pub const PERFORMANCE_FOCUS: &'static str = "Optimize for performance where possible.";

    /// Beginner friendly instruction
    pub const BEGINNER_FRIENDLY: &'static str = "Explain in terms suitable for a beginner.";

    /// Expert level instruction
    pub const EXPERT_LEVEL: &'static str = "Assume expert-level knowledge of the subject.";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_prompts_exist() {
        assert!(!BuiltinPrompts::AGENT_SYSTEM.is_empty());
        assert!(!BuiltinPrompts::CODE_REVIEW.is_empty());
        assert!(!BuiltinPrompts::DEBUG_ERROR.is_empty());
        assert!(!BuiltinPrompts::REFACTOR.is_empty());
    }

    #[test]
    fn test_builtin_prompts_have_placeholders() {
        assert!(BuiltinPrompts::CODE_REVIEW.contains("{{language}}"));
        assert!(BuiltinPrompts::CODE_REVIEW.contains("{{code}}"));
        assert!(BuiltinPrompts::DEBUG_ERROR.contains("{{error_message}}"));
    }

    #[test]
    fn test_prompt_fragments() {
        assert!(!PromptFragments::CONCISE.is_empty());
        assert!(!PromptFragments::STEP_BY_STEP.is_empty());
        assert!(!PromptFragments::JSON_FORMAT.is_empty());
    }

    #[test]
    fn test_git_commit_prompt() {
        assert!(BuiltinPrompts::GIT_COMMIT.contains("conventional commit"));
        assert!(BuiltinPrompts::GIT_COMMIT.contains("{{diff}}"));
    }

    #[test]
    fn test_sql_generation_prompt() {
        assert!(BuiltinPrompts::SQL_GENERATION.contains("{{schema}}"));
        assert!(BuiltinPrompts::SQL_GENERATION.contains("{{dialect}}"));
    }
}

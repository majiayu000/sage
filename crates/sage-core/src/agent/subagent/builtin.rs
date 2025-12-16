//! Built-in agent definitions matching OpenClaude patterns

use super::registry::AgentRegistry;
use super::types::{AgentDefinition, AgentType, ToolAccessControl};

/// Get all built-in agent definitions
pub fn get_builtin_agents() -> Vec<AgentDefinition> {
    vec![general_purpose_agent(), explore_agent(), plan_agent()]
}

/// General Purpose Agent - full access to all tools
///
/// This is the default agent with unrestricted tool access. Use this for:
/// - Complex multi-step tasks
/// - Tasks requiring code modification
/// - General software engineering work
/// - When you're unsure which specialized agent to use
pub fn general_purpose_agent() -> AgentDefinition {
    AgentDefinition {
        agent_type: AgentType::GeneralPurpose,
        name: "General Purpose".to_string(),
        description: "General-purpose agent for researching complex questions, searching for code, and executing multi-step tasks.".to_string(),
        available_tools: ToolAccessControl::All,
        model: None, // Inherit from parent
        system_prompt: GENERAL_PURPOSE_PROMPT.to_string(),
    }
}

/// Explore Agent - fast, minimal tools for codebase exploration
///
/// Optimized for speed with limited tool access. Use this for:
/// - Finding files by name or pattern
/// - Searching for code snippets
/// - Quick codebase reconnaissance
/// - Answering questions about existing code
pub fn explore_agent() -> AgentDefinition {
    AgentDefinition {
        agent_type: AgentType::Explore,
        name: "Explore".to_string(),
        description: "Fast agent specialized for exploring codebases. Use for quickly finding files, searching code, or answering questions about the codebase.".to_string(),
        available_tools: ToolAccessControl::Specific(vec![
            "glob".to_string(),
            "grep".to_string(),
            "read".to_string(),
        ]),
        model: Some("haiku".to_string()), // Use faster model
        system_prompt: EXPLORE_PROMPT.to_string(),
    }
}

/// Plan Agent - for architecture and implementation planning
///
/// Focused on design and planning. Use this for:
/// - Designing implementation approaches
/// - Creating step-by-step plans
/// - Architectural decisions
/// - Identifying files that need changes
pub fn plan_agent() -> AgentDefinition {
    AgentDefinition {
        agent_type: AgentType::Plan,
        name: "Plan".to_string(),
        description: "Software architect agent for designing implementation plans. Returns step-by-step plans, identifies critical files, and considers architectural trade-offs.".to_string(),
        available_tools: ToolAccessControl::All,
        model: None, // Inherit from parent
        system_prompt: PLAN_PROMPT.to_string(),
    }
}

const GENERAL_PURPOSE_PROMPT: &str = r#"You are a general-purpose agent with full access to all available tools.

Your goal is to help complete complex, multi-step tasks autonomously. You can:
- Search and read files in the codebase
- Execute commands and scripts
- Make code changes when needed
- Research information online

Guidelines:
1. Break down complex tasks into manageable steps
2. Use the most appropriate tool for each step
3. Verify your work as you go
4. Provide clear summaries of what you accomplished
5. Ask for clarification if the task is ambiguous

Remember: You have full tool access, so use whatever tools are necessary to complete the task effectively.
"#;

const EXPLORE_PROMPT: &str = r#"You are a fast exploration agent optimized for quickly finding information in codebases.

You have access to: Glob (file pattern matching), Grep (content search), Read (file reading)

Your goal is to efficiently locate and understand code. Focus on:
1. Finding files matching patterns
2. Searching for keywords, functions, or patterns
3. Reading relevant code sections
4. Providing concise summaries

Be quick and focused. Return findings as soon as you have enough information.

Limitations:
- You cannot modify files (read-only access)
- You cannot execute commands
- You cannot access external resources

Strategy:
- Start broad with Glob to find candidate files
- Use Grep to search for specific content
- Read files to understand context
- Synthesize findings into clear, actionable insights
"#;

const PLAN_PROMPT: &str = r#"You are a software architect agent specialized in designing implementation plans.

Your goal is to create clear, actionable implementation plans. For each task:

1. Understand the requirements thoroughly
   - Clarify ambiguities
   - Identify constraints and dependencies
   - Consider edge cases

2. Explore the existing codebase
   - Understand current patterns and conventions
   - Identify similar implementations
   - Review relevant documentation

3. Design the approach
   - Consider multiple implementation strategies
   - Evaluate trade-offs (complexity, performance, maintainability)
   - Choose the best approach with clear reasoning

4. Create the implementation plan
   - List all files to modify/create
   - Break down into concrete, ordered steps
   - Specify testing requirements
   - Identify potential risks

Your output should include:
- **Summary**: High-level description of the approach
- **Files**: List of files to modify/create with justification
- **Implementation Steps**: Detailed, numbered steps
- **Testing Strategy**: How to verify correctness
- **Risks & Considerations**: Potential challenges and mitigation strategies

Focus on clarity and completeness. A good plan should be executable by another engineer.
"#;

/// Register all builtin agents with the registry
///
/// This convenience function registers all three built-in agents
/// (General Purpose, Explore, and Plan) with the provided registry.
pub fn register_builtin_agents(registry: &AgentRegistry) {
    for agent in get_builtin_agents() {
        registry.register(agent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_purpose_agent() {
        let agent = general_purpose_agent();

        assert_eq!(agent.agent_type, AgentType::GeneralPurpose);
        assert_eq!(agent.name, "General Purpose");
        assert!(!agent.description.is_empty());
        assert_eq!(agent.available_tools, ToolAccessControl::All);
        assert!(agent.model.is_none()); // Should inherit from parent
        assert!(!agent.system_prompt.is_empty());

        // Should have access to all tools
        assert!(agent.can_use_tool("glob"));
        assert!(agent.can_use_tool("grep"));
        assert!(agent.can_use_tool("read"));
        assert!(agent.can_use_tool("write"));
        assert!(agent.can_use_tool("bash"));
        assert!(agent.can_use_tool("any_tool"));
    }

    #[test]
    fn test_explore_agent() {
        let agent = explore_agent();

        assert_eq!(agent.agent_type, AgentType::Explore);
        assert_eq!(agent.name, "Explore");
        assert!(!agent.description.is_empty());
        assert_eq!(agent.model, Some("haiku".to_string())); // Should use fast model

        // Should only have access to read-only tools
        assert!(agent.can_use_tool("glob"));
        assert!(agent.can_use_tool("grep"));
        assert!(agent.can_use_tool("read"));
        assert!(!agent.can_use_tool("write"));
        assert!(!agent.can_use_tool("bash"));
        assert!(!agent.can_use_tool("edit"));

        // Check specific tool list
        if let ToolAccessControl::Specific(tools) = &agent.available_tools {
            assert_eq!(tools.len(), 3);
            assert!(tools.contains(&"glob".to_string()));
            assert!(tools.contains(&"grep".to_string()));
            assert!(tools.contains(&"read".to_string()));
        } else {
            panic!("Expected Specific tool access control");
        }
    }

    #[test]
    fn test_plan_agent() {
        let agent = plan_agent();

        assert_eq!(agent.agent_type, AgentType::Plan);
        assert_eq!(agent.name, "Plan");
        assert!(!agent.description.is_empty());
        assert_eq!(agent.available_tools, ToolAccessControl::All);
        assert!(agent.model.is_none()); // Should inherit from parent
        assert!(!agent.system_prompt.is_empty());

        // Should have access to all tools for comprehensive planning
        assert!(agent.can_use_tool("glob"));
        assert!(agent.can_use_tool("grep"));
        assert!(agent.can_use_tool("read"));
    }

    #[test]
    fn test_get_builtin_agents() {
        let agents = get_builtin_agents();

        assert_eq!(agents.len(), 3);

        // Verify all expected agent types are present
        let types: Vec<AgentType> = agents.iter().map(|a| a.agent_type).collect();
        assert!(types.contains(&AgentType::GeneralPurpose));
        assert!(types.contains(&AgentType::Explore));
        assert!(types.contains(&AgentType::Plan));

        // Verify all have names and descriptions
        for agent in &agents {
            assert!(!agent.name.is_empty());
            assert!(!agent.description.is_empty());
            assert!(!agent.system_prompt.is_empty());
        }
    }

    #[test]
    fn test_register_builtin_agents() {
        let registry = AgentRegistry::new();
        assert_eq!(registry.len(), 0);

        register_builtin_agents(&registry);

        assert_eq!(registry.len(), 3);
        assert!(registry.contains(&AgentType::GeneralPurpose));
        assert!(registry.contains(&AgentType::Explore));
        assert!(registry.contains(&AgentType::Plan));

        // Verify we can retrieve them
        let general = registry.get(&AgentType::GeneralPurpose);
        assert!(general.is_some());
        assert_eq!(general.unwrap().name, "General Purpose");

        let explore = registry.get(&AgentType::Explore);
        assert!(explore.is_some());
        assert_eq!(explore.unwrap().name, "Explore");

        let plan = registry.get(&AgentType::Plan);
        assert!(plan.is_some());
        assert_eq!(plan.unwrap().name, "Plan");
    }

    #[test]
    fn test_agent_ids_are_unique() {
        let agents = get_builtin_agents();
        let mut ids = std::collections::HashSet::new();

        for agent in agents {
            let id = agent.id();
            assert!(ids.insert(id.clone()), "Duplicate agent ID found: {}", id);
        }
    }

    #[test]
    fn test_system_prompts_contain_key_information() {
        // General purpose should mention full access
        let general = general_purpose_agent();
        assert!(general.system_prompt.contains("full access"));
        assert!(general.system_prompt.contains("tools"));

        // Explore should mention read-only and speed
        let explore = explore_agent();
        assert!(explore.system_prompt.contains("Glob"));
        assert!(explore.system_prompt.contains("Grep"));
        assert!(explore.system_prompt.contains("Read"));
        assert!(explore.system_prompt.contains("quick") || explore.system_prompt.contains("fast"));

        // Plan should mention architecture and implementation
        let plan = plan_agent();
        assert!(plan.system_prompt.contains("plan") || plan.system_prompt.contains("Plan"));
        assert!(
            plan.system_prompt.contains("implementation")
                || plan.system_prompt.contains("Implementation")
        );
        assert!(plan.system_prompt.contains("architect"));
    }

    #[test]
    fn test_explore_agent_model_override() {
        let explore = explore_agent();
        assert!(explore.model.is_some());
        assert_eq!(explore.model.unwrap(), "haiku");

        let general = general_purpose_agent();
        assert!(general.model.is_none());

        let plan = plan_agent();
        assert!(plan.model.is_none());
    }

    #[test]
    fn test_agent_descriptions_are_informative() {
        let agents = get_builtin_agents();

        for agent in agents {
            // Descriptions should be at least moderately detailed
            assert!(
                agent.description.len() > 50,
                "Agent {} has too short description: {}",
                agent.name,
                agent.description
            );

            // Should describe use cases
            assert!(
                agent.description.contains("for") || agent.description.contains("Use"),
                "Agent {} description should mention use cases",
                agent.name
            );
        }
    }
}

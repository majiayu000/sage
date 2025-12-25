//! Task tool schema definition

use sage_core::tools::types::ToolSchema;
use serde_json::json;

/// Generate the tool schema for the Task tool
pub fn task_tool_schema() -> ToolSchema {
    ToolSchema {
        name: "Task".to_string(),
        description: task_tool_description().to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short (3-5 word) description of the task"
                },
                "prompt": {
                    "type": "string",
                    "description": "The task for the agent to perform"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "The type of specialized agent to use (general-purpose, Explore, Plan)"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model to use (sonnet, opus, haiku). Defaults to inherit from parent.",
                    "enum": ["sonnet", "opus", "haiku"]
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run this agent in the background. Use TaskOutput to read output later.",
                    "default": false
                },
                "resume": {
                    "type": "string",
                    "description": "Optional agent ID to resume from. Agent continues with previous context preserved."
                },
                "thoroughness": {
                    "type": "string",
                    "description": "Thoroughness level for Explore agents: quick (fast, 5 steps), medium (balanced, 15 steps), very_thorough (comprehensive, 30 steps). Default: medium.",
                    "enum": ["quick", "medium", "very_thorough"],
                    "default": "medium"
                }
            },
            "required": ["description", "prompt", "subagent_type"]
        }),
    }
}

/// Get the tool description
pub fn task_tool_description() -> &'static str {
    r#"Launch a new agent to handle complex, multi-step tasks autonomously.

The Task tool launches specialized agents (subprocesses) that autonomously handle complex tasks. Each agent type has specific capabilities and tools available to it.

Available agent types:
- general-purpose: General-purpose agent with access to all tools. Use for complex multi-step tasks.
- Explore: Fast agent for codebase exploration. Use for finding files, searching code, or answering questions about the codebase. (Tools: Glob, Grep, Read, Bash). Supports thoroughness levels: "quick", "medium", "very_thorough".
- Plan: Software architect agent for designing implementation plans. Returns step-by-step plans and identifies critical files. (Tools: All)

When NOT to use the Task tool:
- If you want to read a specific file path, use Read or Glob instead
- If searching for a specific class definition, use Glob instead
- If searching code within 2-3 specific files, use Read instead

Usage notes:
- Launch multiple agents concurrently when possible (use single message with multiple tool calls)
- Agent results are not visible to the user - summarize results in your response
- Use run_in_background=true for background execution, then use TaskOutput to retrieve results
- Use resume parameter with agent ID to continue previous execution
- For Explore agents, specify thoroughness: "quick" (5 steps), "medium" (15 steps), or "very_thorough" (30 steps)"#
}

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
- general-purpose: General-purpose agent with access to all tools. Use for complex multi-step tasks that require writing code or making changes.
- Explore: Fast agent for codebase exploration. Use for finding files, searching code, or answering questions about the codebase. (Tools: Glob, Grep, Read, Bash). Supports thoroughness levels: "quick", "medium", "very_thorough".
- Plan: Software architect agent for designing implementation plans. Returns step-by-step plans and identifies critical files. (Tools: All)

When NOT to use the Task tool:
- If you want to read a specific file path, use Read or Glob instead
- If searching for a specific class definition like "class Foo", use Glob instead
- If searching code within a specific file or set of 2-3 files, use Read instead

Usage notes:
- Always include a short description (3-5 words) summarizing what the agent will do
- Launch multiple agents concurrently whenever possible, to maximize performance; to do that, use a single message with multiple tool uses
- When the agent is done, it will return a single message back to you. The result returned by the agent is not visible to the user. To show the user the result, you should send a text message back to the user with a concise summary of the result.
- You can optionally run agents in the background using the run_in_background parameter. When an agent runs in the background, you will need to use TaskOutput to retrieve its results once it's done.
- Agents can be resumed using the `resume` parameter by passing the agent ID from a previous invocation. When resumed, the agent continues with its full previous context preserved.
- Provide clear, detailed prompts so the agent can work autonomously and return exactly the information you need.
- The agent's outputs should generally be trusted
- Clearly tell the agent whether you expect it to write code or just to do research (search, file reads, web fetches, etc.), since it is not aware of the user's intent

Example usage:

<example>
user: "Please write a function that checks if a number is prime"
assistant: [Writes the function using Edit tool]
assistant: Now let me use the Task tool to launch an Explore agent to verify there are no similar functions in the codebase
</example>

<example>
user: "Where are errors from the client handled?"
assistant: [Uses the Task tool with subagent_type=Explore to find the files that handle client errors]
</example>

<example>
user: "What is the codebase structure?"
assistant: [Uses the Task tool with subagent_type=Explore]
</example>"#
}

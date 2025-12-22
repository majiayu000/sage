# Explore Agent Usage Guide

The Explore Agent is a specialized sub-agent optimized for fast codebase exploration and search tasks. It follows Claude Code's design principles with restricted tool access for safety and performance.

## When to Use Explore Agent

Use the Explore Agent when you need to:

1. **Find files by pattern**: Locate files matching specific naming conventions
2. **Search code content**: Find specific functions, classes, or patterns in code
3. **Quick reconnaissance**: Understand codebase structure without modifications
4. **Answer questions**: Get information about existing code

## When NOT to Use Explore Agent

Don't use Explore Agent for:

- ❌ Modifying files (use General Purpose agent instead)
- ❌ Creating new files
- ❌ Running tests or builds
- ❌ Installing dependencies
- ❌ Making git commits

## Tool Access

The Explore Agent has access to only these tools:

- **Glob**: Find files by pattern (e.g., `**/*.rs`, `src/**/*.ts`)
- **Grep**: Search file contents with regex
- **Read**: Read specific files
- **Bash**: Read-only commands only (ls, git status, git log, git diff, find, cat, head, tail)

## Usage Examples

### Example 1: Find all test files

```rust
use sage_core::agent::subagent::{SubAgentExecutor, AgentType};

let executor = SubAgentExecutor::new(config)?;
let result = executor.execute(
    AgentType::Explore,
    "Find all test files in the codebase",
    None, // thoroughness: quick
).await?;
```

### Example 2: Search for specific function

```rust
let result = executor.execute(
    AgentType::Explore,
    "Find where the `execute_task` function is defined",
    Some("medium"), // thoroughness: medium
).await?;
```

### Example 3: Understand module structure

```rust
let result = executor.execute(
    AgentType::Explore,
    "What files are in the llm module and what do they do?",
    Some("very thorough"),
).await?;
```

## Thoroughness Levels

Specify how thorough the exploration should be:

- **"quick"**: Basic search, minimal file reads
- **"medium"**: Moderate exploration, read key files
- **"very thorough"**: Comprehensive analysis, read multiple files

## Performance Characteristics

- **Model**: Uses Haiku (fast, cost-effective)
- **Speed**: Optimized for quick responses
- **Parallel**: Makes multiple tool calls in parallel
- **Read-only**: No file modifications = safe to run

## Integration with Task Tool

The Explore Agent is automatically available through the Task tool:

```
When the user asks to explore the codebase, use:
- Task tool with subagent_type='Explore'
- Specify thoroughness level in the prompt
```

## Safety Features

1. **Read-only mode**: Cannot modify any files
2. **Tool restrictions**: Only has access to search/read tools
3. **Bash restrictions**: Can only run read-only bash commands
4. **No side effects**: Safe to run without worrying about changes

## Best Practices

1. **Be specific**: Clear search queries get better results
2. **Use thoroughness**: Adjust based on task complexity
3. **Parallel searches**: Agent will automatically parallelize when possible
4. **Absolute paths**: Agent returns absolute file paths for easy navigation

## Example Output

```
Found 3 relevant files:

1. /path/to/crates/sage-core/src/llm/client.rs:242
   - Contains the main `chat()` method for LLM requests
   - Implements retry logic and rate limiting

2. /path/to/crates/sage-core/src/llm/fallback.rs:241
   - Implements model-level fallback chain
   - Handles failure detection and recovery

3. /path/to/crates/sage-core/src/llm/provider_fallback.rs:35
   - Implements provider-level fallback
   - Switches between providers on quota errors
```

## Comparison with General Purpose Agent

| Feature | Explore Agent | General Purpose Agent |
|---------|--------------|----------------------|
| Speed | Fast (Haiku) | Slower (Sonnet) |
| Tools | Limited (4 tools) | All tools |
| Modifications | Read-only | Can modify files |
| Use case | Search & explore | Full implementation |
| Cost | Low | Higher |

## Implementation Details

The Explore Agent is defined in:
- `crates/sage-core/src/agent/subagent/builtin.rs` - Agent definition
- `crates/sage-core/src/prompts/agent_prompts/mod.rs` - System prompt
- `crates/sage-core/src/agent/subagent/executor.rs` - Execution logic

## Testing

Test the Explore Agent:

```bash
cargo test --lib explore_agent
```

## Troubleshooting

### Agent tries to modify files

This shouldn't happen - the agent doesn't have access to Write or Edit tools. If you see this, it's a bug.

### Agent is too slow

Try using "quick" thoroughness level or be more specific in your query.

### Agent can't find files

Make sure:
1. Files exist in the working directory
2. Query is specific enough
3. Using correct glob patterns

## Future Enhancements

Potential improvements:
- [ ] Caching of search results
- [ ] Semantic code search
- [ ] Integration with LSP for symbol lookup
- [ ] Incremental search refinement

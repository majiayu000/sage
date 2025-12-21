# Slash Commands

Sage Agent supports Claude Code-style slash commands that provide quick access to common operations. These commands work in both `sage run` and `sage interactive` modes.

## Command Types

| Type | Behavior | Examples |
|------|----------|----------|
| **Interactive** | Opens UI for user selection | `/resume` |
| **Local** | Displays output directly, no LLM call | `/cost`, `/status`, `/commands` |
| **Prompt** | Expands to prompt sent to LLM | `/undo`, `/help`, `/plan` |
| **Special** | Custom handling | `/clear` |

## Built-in Commands

### Session Management

#### `/resume`
Resume a previous conversation session.

```bash
# Interactive session selector
sage run "/resume"

# Resume specific session by ID
sage run "/resume abc123-session-id"

# Show sessions from all projects
sage run "/resume --all"
```

**Features:**
- Fuzzy search through sessions
- Shows session metadata (model, messages, branch)
- Displays recent conversation preview
- Cross-project session detection

#### `/clear`
Clear conversation history and start fresh.

```bash
sage run "/clear"
```

In interactive mode, this resets the conversation context.

#### `/compact`
Summarize and compact the conversation to save tokens.

```bash
sage run "/compact"
```

### Information Commands

#### `/cost`
Show session cost and token usage.

```bash
sage run "/cost"
```

**Output includes:**
- Total tokens used
- Cache creation/read tokens
- Estimated cost (requires trajectory)

#### `/context`
Show context window usage breakdown.

```bash
sage run "/context"
```

**Output includes:**
- System prompt tokens
- Conversation history tokens
- Tool definition tokens
- Available remaining tokens

#### `/status`
Show agent status and version.

```bash
sage run "/status"
```

**Output includes:**
- Sage Agent version
- Current provider and model
- Number of registered commands
- Configuration status

#### `/commands`
List all available slash commands.

```bash
sage run "/commands"
```

**Grouped by:**
- Built-in Commands
- Project Commands (`.sage/commands/`)
- User Commands (`~/.config/sage/commands/`)

### File Operations

#### `/undo`
Undo the last file changes using git restore.

```bash
sage run "/undo"
```

**Process:**
1. Runs `git status` to find changes
2. Runs `git diff` to show changes
3. Uses `git restore` to revert files
4. Only affects current working directory

#### `/checkpoint [name]`
Create a state checkpoint.

```bash
# Create checkpoint with auto-generated name
sage run "/checkpoint"

# Create named checkpoint
sage run "/checkpoint my-save-point"
```

#### `/restore [checkpoint-id]`
Restore to a previous checkpoint.

```bash
# List available checkpoints
sage run "/restore"

# Restore specific checkpoint
sage run "/restore my-save-point"
```

### Planning

#### `/plan`
View or manage execution plan.

```bash
# View current plan
sage run "/plan"

# Open plan in editor
sage run "/plan open"

# Clear the plan
sage run "/plan clear"

# Create new plan
sage run "/plan create"
```

Plan files are stored at `.sage/plan.md`.

### Configuration

#### `/config`
Show or modify configuration.

```bash
# Show current config
sage run "/config"

# Modify settings
sage run "/config max_steps=30"
```

#### `/init`
Initialize .sage directory with default configuration.

```bash
sage run "/init"
```

Creates:
- `.sage/settings.json`
- `.sage/commands/` directory

### Other Commands

#### `/help`
Show AI help information.

```bash
sage run "/help"
```

#### `/tasks`
List running and completed background tasks.

```bash
sage run "/tasks"
```

## Custom Commands

Create custom slash commands by adding markdown files to:
- **Project-level:** `.sage/commands/*.md`
- **User-level:** `~/.config/sage/commands/*.md`

### Command File Format

```markdown
---
name: review
description: Review code changes
arguments:
  - name: file
    required: true
    description: File to review
---

Please review the following code:
$ARGUMENTS

Focus on:
1. Code quality
2. Potential bugs
3. Performance issues
```

### Template Variables

| Variable | Description |
|----------|-------------|
| `$ARGUMENTS` | All arguments joined with spaces |
| `$ARGUMENTS_JSON` | Arguments as JSON array |
| `$ARG1`, `$ARG2`, ... | Individual arguments by position |

### Example Custom Commands

#### Code Review
`.sage/commands/review.md`:
```markdown
---
name: review
description: Review code for quality and issues
---

Review the following code changes:
$ARGUMENTS

Check for:
- Bug potential
- Security issues
- Performance problems
- Code style violations
```

Usage:
```bash
sage run "/review src/main.rs"
```

#### Test Generator
`.sage/commands/test.md`:
```markdown
---
name: test
description: Generate tests for a file
---

Generate comprehensive tests for: $ARG1

Include:
- Unit tests
- Edge cases
- Error handling tests
```

Usage:
```bash
sage run "/test src/lib.rs"
```

## Using in Interactive Mode

In `sage interactive`, all slash commands are available:

```
> help
[Shows built-in commands including slash commands]

> /resume
[Opens session selector]

> /cost
Session Cost & Usage
====================
...

> /undo
[Agent runs git restore]
```

## Command Discovery

Commands are automatically discovered from:
1. Built-in commands (16 total)
2. `.sage/commands/` in current directory
3. `~/.config/sage/commands/` for user-level commands

Use `/commands` to see all available commands in your current context.

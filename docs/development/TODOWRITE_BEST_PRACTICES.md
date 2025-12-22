# TodoWrite Best Practices

This document provides guidelines for systematic use of the TodoWrite tool to ensure effective task tracking and progress visibility.

## Core Principles

1. **Proactive Usage**: Create todo lists at the start of complex tasks
2. **Real-time Updates**: Mark tasks as in_progress before starting, completed immediately after finishing
3. **Single Focus**: Only ONE task should be in_progress at a time
4. **Granular Tasks**: Break down complex work into specific, actionable items

## When to Use TodoWrite

### ✅ Use TodoWrite for:

1. **Complex Multi-step Tasks** (3+ steps)
   ```
   User: "Add dark mode toggle to the application"
   → Create todo list with: UI component, state management, CSS, testing
   ```

2. **Multiple Related Tasks**
   ```
   User: "Implement user registration, product catalog, shopping cart"
   → Create todo list with each feature as separate task
   ```

3. **Non-trivial Implementation**
   ```
   User: "Optimize React application performance"
   → Create todo list: Profile, identify bottlenecks, implement fixes, verify
   ```

4. **User-provided Lists**
   ```
   User: "Please do: 1) fix bug, 2) add tests, 3) update docs"
   → Create todo list matching user's structure
   ```

### ❌ Don't Use TodoWrite for:

1. **Single Simple Tasks**
   ```
   User: "Fix typo in README"
   → Just fix it directly, no todo list needed
   ```

2. **Trivial Operations**
   ```
   User: "Run npm install"
   → Execute directly, no tracking needed
   ```

3. **Pure Information Requests**
   ```
   User: "What does this function do?"
   → Answer directly, no todo list
   ```

## Task Lifecycle

### 1. Creation

Create todos at the START of work:

```rust
// Good: Create todos before starting
TodoWrite {
    todos: [
        { content: "Analyze codebase structure", status: "pending", activeForm: "Analyzing codebase structure" },
        { content: "Design implementation approach", status: "pending", activeForm: "Designing implementation approach" },
        { content: "Implement core functionality", status: "pending", activeForm: "Implementing core functionality" },
        { content: "Add tests", status: "pending", activeForm: "Adding tests" },
    ]
}
```

### 2. Starting Work

Mark task as `in_progress` BEFORE starting:

```rust
// Good: Mark in_progress before starting
TodoWrite {
    todos: [
        { content: "Analyze codebase structure", status: "in_progress", ... },
        { content: "Design implementation approach", status: "pending", ... },
        ...
    ]
}
// Then start the actual work
```

### 3. Completing Work

Mark task as `completed` IMMEDIATELY after finishing:

```rust
// Good: Mark completed right after finishing
TodoWrite {
    todos: [
        { content: "Analyze codebase structure", status: "completed", ... },
        { content: "Design implementation approach", status: "in_progress", ... },
        ...
    ]
}
```

### 4. Adapting

Add/remove tasks as you discover new requirements:

```rust
// Good: Adapt todo list based on findings
TodoWrite {
    todos: [
        { content: "Analyze codebase structure", status: "completed", ... },
        { content: "Design implementation approach", status: "completed", ... },
        { content: "Refactor existing code", status: "pending", ... }, // NEW: discovered during analysis
        { content: "Implement core functionality", status: "pending", ... },
        { content: "Add tests", status: "pending", ... },
    ]
}
```

## Task Description Guidelines

### Content (Imperative Form)

Use clear, actionable verbs:

- ✅ "Implement user authentication"
- ✅ "Fix memory leak in cache"
- ✅ "Add validation to form inputs"
- ❌ "Authentication" (too vague)
- ❌ "Working on cache" (not imperative)

### Active Form (Present Continuous)

Match the content but in present continuous:

- Content: "Implement user authentication"
  - Active: "Implementing user authentication"
- Content: "Fix memory leak in cache"
  - Active: "Fixing memory leak in cache"
- Content: "Add validation to form inputs"
  - Active: "Adding validation to form inputs"

## Common Patterns

### Pattern 1: Feature Implementation

```rust
TodoWrite {
    todos: [
        { content: "Research existing patterns", status: "pending", activeForm: "Researching existing patterns" },
        { content: "Design API interface", status: "pending", activeForm: "Designing API interface" },
        { content: "Implement core logic", status: "pending", activeForm: "Implementing core logic" },
        { content: "Add error handling", status: "pending", activeForm: "Adding error handling" },
        { content: "Write tests", status: "pending", activeForm: "Writing tests" },
        { content: "Update documentation", status: "pending", activeForm: "Updating documentation" },
    ]
}
```

### Pattern 2: Bug Fix

```rust
TodoWrite {
    todos: [
        { content: "Reproduce the bug", status: "pending", activeForm: "Reproducing the bug" },
        { content: "Identify root cause", status: "pending", activeForm: "Identifying root cause" },
        { content: "Implement fix", status: "pending", activeForm: "Implementing fix" },
        { content: "Verify fix works", status: "pending", activeForm: "Verifying fix works" },
        { content: "Add regression test", status: "pending", activeForm: "Adding regression test" },
    ]
}
```

### Pattern 3: Refactoring

```rust
TodoWrite {
    todos: [
        { content: "Analyze current implementation", status: "pending", activeForm: "Analyzing current implementation" },
        { content: "Identify improvement opportunities", status: "pending", activeForm: "Identifying improvement opportunities" },
        { content: "Create refactoring plan", status: "pending", activeForm: "Creating refactoring plan" },
        { content: "Refactor code incrementally", status: "pending", activeForm: "Refactoring code incrementally" },
        { content: "Verify tests still pass", status: "pending", activeForm: "Verifying tests still pass" },
    ]
}
```

## Anti-patterns

### ❌ Batching Completions

```rust
// Bad: Marking multiple tasks as completed at once
TodoWrite {
    todos: [
        { content: "Task 1", status: "completed", ... },
        { content: "Task 2", status: "completed", ... },
        { content: "Task 3", status: "completed", ... },
    ]
}
```

**Why bad**: Loses visibility into progress. User can't see what you're working on.

**Fix**: Mark each task completed immediately after finishing it.

### ❌ Multiple In-Progress Tasks

```rust
// Bad: Multiple tasks marked as in_progress
TodoWrite {
    todos: [
        { content: "Task 1", status: "in_progress", ... },
        { content: "Task 2", status: "in_progress", ... },
    ]
}
```

**Why bad**: Unclear what you're actually working on. Confusing for user.

**Fix**: Only ONE task should be in_progress at a time.

### ❌ Vague Task Descriptions

```rust
// Bad: Vague descriptions
TodoWrite {
    todos: [
        { content: "Fix stuff", status: "pending", ... },
        { content: "Update things", status: "pending", ... },
    ]
}
```

**Why bad**: User doesn't understand what you're doing.

**Fix**: Be specific about what each task involves.

### ❌ Forgetting to Update

```rust
// Bad: Created todo list but never updated it
// (Initial state)
TodoWrite { todos: [...all pending...] }
// (After completing all work)
// No updates! User has no idea what happened.
```

**Why bad**: Todo list becomes stale and useless.

**Fix**: Update todo list as you progress through tasks.

## Integration with Agent Workflow

### Example: Complete Workflow

```
1. User Request: "Add user authentication to the app"

2. Create Todo List:
   TodoWrite { todos: [
       { content: "Analyze existing auth patterns", status: "pending", ... },
       { content: "Design auth flow", status: "pending", ... },
       { content: "Implement auth logic", status: "pending", ... },
       { content: "Add tests", status: "pending", ... },
   ]}

3. Start First Task:
   TodoWrite { todos: [
       { content: "Analyze existing auth patterns", status: "in_progress", ... },
       ...
   ]}
   [Do the analysis work]

4. Complete First Task:
   TodoWrite { todos: [
       { content: "Analyze existing auth patterns", status: "completed", ... },
       { content: "Design auth flow", status: "in_progress", ... },
       ...
   ]}
   [Do the design work]

5. Continue until all tasks completed...

6. Final State:
   TodoWrite { todos: [
       { content: "Analyze existing auth patterns", status: "completed", ... },
       { content: "Design auth flow", status: "completed", ... },
       { content: "Implement auth logic", status: "completed", ... },
       { content: "Add tests", status: "completed", ... },
   ]}
```

## Benefits of Systematic Usage

1. **Transparency**: User always knows what you're working on
2. **Progress Tracking**: Clear visibility into completion status
3. **Accountability**: Ensures all requirements are addressed
4. **Planning**: Forces you to think through the full scope upfront
5. **Communication**: Provides structure for status updates

## Validation Rules

The system should validate:

1. **Single In-Progress**: Only one task can be in_progress
2. **Required Fields**: All tasks must have content, status, and activeForm
3. **Status Transitions**: Tasks should progress logically (pending → in_progress → completed)
4. **Non-empty Content**: Task descriptions must be meaningful

## Monitoring and Metrics

Track these metrics for quality:

- **Todo List Usage Rate**: % of complex tasks that use TodoWrite
- **Update Frequency**: How often todo lists are updated
- **Completion Rate**: % of tasks marked as completed
- **Task Granularity**: Average number of tasks per todo list

## Future Enhancements

Potential improvements:

- [ ] Automatic task breakdown suggestions
- [ ] Time estimates per task
- [ ] Dependency tracking between tasks
- [ ] Automatic progress reporting
- [ ] Integration with trajectory recording

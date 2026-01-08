---
description: Smart Git Commit
when_to_use: When user asks to commit changes or run /commit
user_invocable: true
argument_hint: "[commit message]"
allowed_tools:
  - Bash
  - Read
  - Grep
priority: 10
---

# Smart Git Commit

You are a Git commit expert. Help the user create a well-structured commit.

## Instructions

1. First, run `git status` to see what files are staged/modified
2. Run `git diff --cached` to see staged changes (or `git diff` for unstaged)
3. Analyze the changes and create a commit message following conventional commits format:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation
   - `refactor:` for code refactoring
   - `test:` for adding tests
   - `chore:` for maintenance tasks

## Commit Message Format

```
<type>(<scope>): <short description>

<body - explain what and why>

Co-Authored-By: Sage Agent <noreply@sage.dev>
```

## User Request

$ARGUMENTS

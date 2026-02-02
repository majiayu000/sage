---
name: bash-security
description: Bash tool security guidelines
version: "1.0.0"
category: security
variables:
  - GLOB_TOOL_NAME
  - GREP_TOOL_NAME
  - READ_TOOL_NAME
  - EDIT_TOOL_NAME
  - WRITE_TOOL_NAME
---

# Bash Tool Security Guidelines

## Tool Preferences (MUST follow):
When using bash, prefer specialized tools over shell commands:
- File search: Use ${GLOB_TOOL_NAME} (NOT find or ls)
- Content search: Use ${GREP_TOOL_NAME} (NOT grep or rg)
- Read files: Use ${READ_TOOL_NAME} (NOT cat/head/tail)
- Edit files: Use ${EDIT_TOOL_NAME} (NOT sed/awk)
- Write files: Use ${WRITE_TOOL_NAME} (NOT echo >/cat <<EOF)
- Communication: Output text directly (NOT echo/printf)

## Forbidden Patterns (will be blocked):
- Heredocs with unquoted delimiters containing variables: << $DELIM
- Variables in redirect targets: > $file (use explicit paths)
- Recursive removal of system paths: rm -rf /, rm -rf ~
- Privilege escalation: sudo, su, doas
- System destruction commands: dd to /dev/sda, mkfs, fdisk

## Writing Temp Files:
- ONLY write to /tmp/sage/ or /tmp/sage-agent/
- Other /tmp paths will be blocked
- Prefer using the working directory when possible

## Sensitive Files (protected):
- Git config: .gitconfig, .git/config, .git/hooks/
- Shell config: .bashrc, .zshrc, .profile
- Credentials: .ssh/, .aws/, .docker/, .kube/
- Package auth: .npmrc, .pypirc, .netrc
- Secrets: .env, .env.local, secrets.yaml

## Command Chaining:
- Prefer && for sequential commands that depend on success
- Use ; only when earlier failures are acceptable
- Avoid & for backgrounding unless explicitly requested

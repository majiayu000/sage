# Permission System Improvement Plan

## Background

Based on analysis of open-claude-code's permission system, we identified significant improvements needed for Sage's user interaction and permission handling.

## Current Issues

1. **AI-dependent confirmation**: Dangerous commands require AI to call `ask_user_question`, but AI may forget or ignore this
2. **No permission persistence**: Users cannot save "always allow" or "always deny" rules
3. **No automatic blocking**: Tool layer doesn't automatically block dangerous operations

## Proposed Improvements

### Phase 1: Tool-level Permission Checking (Priority: High)

Reference: open-claude-code's `checkPermissions()` mechanism

#### 1.1 Permission Behavior Types
```rust
pub enum PermissionBehavior {
    Allow,  // Auto-allow execution
    Deny,   // Reject execution
    Ask,    // Show confirmation dialog, wait for user
}
```

#### 1.2 Permission Check Result
```rust
pub struct PermissionCheckResult {
    pub behavior: PermissionBehavior,
    pub message: String,
    pub reason: PermissionDecisionReason,
}

pub enum PermissionDecisionReason {
    Rule(PermissionRule),
    Hook,
    Default,
}
```

#### 1.3 Tool Trait Extension
```rust
trait Tool {
    // Existing methods...

    /// Check permissions before execution
    /// Returns behavior that determines how to proceed
    async fn check_permissions(&self, call: &ToolCall) -> PermissionCheckResult;
}
```

### Phase 2: Permission Rules Storage (Priority: Medium)

#### 2.1 Rule Types
```rust
pub struct PermissionRule {
    pub id: String,
    pub tool_name: String,
    pub pattern: String,      // e.g., "rm *", "git push --force"
    pub behavior: PermissionBehavior,
    pub created_at: DateTime,
}
```

#### 2.2 Rule Storage
- Store in `~/.sage/permissions.json`
- Three categories:
  - `always_allow_rules`
  - `always_deny_rules`
  - `always_ask_rules`

### Phase 3: Permission UI Components (Priority: Medium)

#### 3.1 Permission Dialog
When `behavior == Ask`:
```
┌─────────────────────────────────────────────┐
│  Permission Required                        │
│                                             │
│  Tool: bash                                 │
│  Command: rm -rf ./build                    │
│                                             │
│  This will delete files recursively.        │
│                                             │
│  [1] Yes, execute once                      │
│  [2] Yes, always allow this pattern         │
│  [3] No, reject                             │
│  [4] No, always deny this pattern           │
│                                             │
│  Choice: _                                  │
└─────────────────────────────────────────────┘
```

#### 3.2 User Feedback on Rejection
Allow user to provide feedback when rejecting:
```
Rejection reason (optional): _
```
This feedback is sent back to the AI.

### Phase 4: Execution Flow Changes (Priority: High)

#### 4.1 New Execution Flow
```
Tool Call
    │
    ▼
check_permissions()
    │
    ├─► behavior=Allow ──► Execute tool
    │
    ├─► behavior=Deny ───► Return rejection message to AI
    │
    └─► behavior=Ask ────► Show permission dialog
                              │
                              ├─► User allows ──► Execute tool
                              │                   (optionally save rule)
                              │
                              └─► User denies ──► Return rejection to AI
                                                  (optionally save rule)
```

#### 4.2 Integration Points
- `crates/sage-core/src/tools/executor.rs`: Add permission check before execution
- `crates/sage-core/src/agent/unified/step_execution.rs`: Handle permission UI
- New: `crates/sage-core/src/permissions/` module

### Phase 5: Dangerous Command Detection (Priority: High)

#### 5.1 Bash Tool Patterns
Already partially implemented. Expand to include:
- `rm` - file deletion
- `rmdir` - directory deletion
- `git push --force` - force push
- `git reset --hard` - discard changes
- `DROP DATABASE/TABLE` - database destruction
- `docker rm/prune` - container removal
- `chmod 777` - insecure permissions
- `curl | sh` - pipe to shell

#### 5.2 File Tool Patterns
- Overwriting existing files
- Deleting files
- Modifying system files

### Implementation Order

1. **Immediate** (already done):
   - [x] System prompt update for ask_user_question requirement
   - [x] Bash tool `user_confirmed` parameter
   - [x] `ConfirmationRequired` error type

2. **Next Sprint**:
   - [ ] Permission behavior enum and types
   - [ ] Permission check trait method
   - [ ] Basic permission dialog UI

3. **Future**:
   - [ ] Permission rules storage
   - [ ] "Remember my choice" functionality
   - [ ] Hook-based permission control

## Files to Create/Modify

### New Files
- `crates/sage-core/src/permissions/mod.rs`
- `crates/sage-core/src/permissions/types.rs`
- `crates/sage-core/src/permissions/rules.rs`
- `crates/sage-core/src/permissions/storage.rs`
- `crates/sage-core/src/ui/permission_dialog.rs`

### Modified Files
- `crates/sage-core/src/tools/base/tool_trait.rs` - Add check_permissions method
- `crates/sage-core/src/tools/executor.rs` - Integrate permission checking
- `crates/sage-core/src/agent/unified/step_execution.rs` - Handle permission UI
- `crates/sage-tools/src/tools/process/bash.rs` - Use new permission system

## References

- open-claude-code permission system analysis
- Claude Code's three-layer permission model (allow/deny/ask)

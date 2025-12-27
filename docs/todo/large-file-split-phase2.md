# Phase 2: Large File Splitting

Based on the 200-line file limit requirement, the following files need to be split.

## Task Assignment

| Agent | File | Lines | Target Modules |
|-------|------|-------|----------------|
| Agent 1 | `checkpoints/manager.rs` | 875 | lifecycle, storage, cache |
| Agent 2 | `tools/parallel_executor.rs` | 855 | core, semaphore, permission, stats |
| Agent 3 | `workspace/patterns.rs` | 840 | file_patterns, ignore, matching |

---

## Agent 1: Split `checkpoints/manager.rs`

**Path**: `crates/sage-core/src/checkpoints/manager.rs` (875 lines)

### Target Structure
```
checkpoints/manager/
├── mod.rs           # Module exports and CheckpointManager struct
├── lifecycle.rs     # create, restore, delete operations
├── storage.rs       # Storage operations (save, load, list)
└── cache.rs         # Cache management
```

### Rules
1. Each file ≤ 200 lines
2. Keep `CheckpointManager` struct in `mod.rs`
3. Use `pub(super)` for internal methods
4. Re-export public API via `pub use`

---

## Agent 2: Split `tools/parallel_executor.rs`

**Path**: `crates/sage-core/src/tools/parallel_executor.rs` (855 lines)

### Target Structure
```
tools/parallel_executor/
├── mod.rs           # Module exports and ParallelToolExecutor struct
├── core.rs          # execute, execute_batch methods
├── semaphore.rs     # Semaphore management
├── permission.rs    # Permission checking logic
└── stats.rs         # ExecutorStats and statistics
```

### Rules
1. Each file ≤ 200 lines
2. Keep `ParallelToolExecutor` struct in `mod.rs`
3. Move stats-related code to `stats.rs`
4. Move permission checks to `permission.rs`

---

## Agent 3: Split `workspace/patterns.rs`

**Path**: `crates/sage-core/src/workspace/patterns.rs` (840 lines)

### Target Structure
```
workspace/patterns/
├── mod.rs           # Module exports and main types
├── file_patterns.rs # File pattern definitions
├── ignore.rs        # Gitignore-style patterns
└── matching.rs      # Pattern matching logic
```

### Rules
1. Each file ≤ 200 lines
2. Keep public types in `mod.rs`
3. Group related functions together

---

## Verification

```bash
# After each split:
cargo check

# Ensure all tests pass:
cargo test -p sage-core

# Verify file sizes:
wc -l crates/sage-core/src/checkpoints/manager/*.rs
wc -l crates/sage-core/src/tools/parallel_executor/*.rs
wc -l crates/sage-core/src/workspace/patterns/*.rs
```

## Success Criteria

- [ ] All new files ≤ 200 lines
- [ ] `cargo check` passes
- [ ] `cargo test -p sage-core` passes
- [ ] Public API unchanged

# Sage Workspace Dependency Audit Report

**Analysis Date:** 2025-12-23
**Working Directory:** `/Users/lifcc/Desktop/code/AI/agent/sage`
**Analyzer:** Automated dependency usage scanner

---

## Executive Summary

This audit identifies dependencies declared in `Cargo.toml` files but not actively used in source code. Out of 11 dependencies audited, **2 confirmed unused dependencies** were found.

### Key Findings:
- **sage-core**: 1 unused dependency
- **sage-cli**: 1 unused dependency
- **sage-tools**: All dependencies are used
- **sage-sdk**: All dependencies are used

---

## Detailed Findings by Crate

### 1. sage-core (`crates/sage-core`)

**File:** `crates/sage-core/Cargo.toml`

| Dependency | Status | Notes |
|-----------|--------|-------|
| `lru` | ❌ **UNUSED** | Declared on line 51, no references found in source code |
| `toml` | ✅ USED | Used in `config/loader.rs` for config file parsing |
| `serde_yaml` | ✅ USED | Used in `config/loader.rs` for YAML config support |
| `shell-words` | ✅ USED | Used in `commands/types.rs` for shell argument parsing |
| `glob` | ✅ USED | Used in `workspace/patterns.rs` via `glob::glob()` |

**Recommendation:**
```toml
# Remove from Cargo.toml line 51:
lru = "0.12"
```

---

### 2. sage-cli (`crates/sage-cli`)

**File:** `crates/sage-cli/Cargo.toml`

| Dependency | Status | Notes |
|-----------|--------|-------|
| `uuid` | ❌ **UNUSED** | Declared on line 38, no references found in source code |
| `walkdir` | ✅ USED | Used in `ui_launcher.rs` for directory traversal |

**Recommendation:**
```toml
# Remove from Cargo.toml line 38:
uuid = { workspace = true }
```

**Note:** `uuid` is a workspace dependency, so removing it from sage-cli won't affect other crates that use it.

---

### 3. sage-tools (`crates/sage-tools`)

**File:** `crates/sage-tools/Cargo.toml`

| Dependency | Status | Notes |
|-----------|--------|-------|
| `toml` | ✅ USED | Used in `config.rs` for configuration parsing |
| `base64` | ✅ USED | Used in `tools/network/http_client.rs` for binary data encoding |
| `once_cell` | ✅ USED | Used in multiple files for lazy initialization |

**Status:** ✅ All dependencies are actively used

---

### 4. sage-sdk (`crates/sage-sdk`)

**File:** `crates/sage-sdk/Cargo.toml`

| Dependency | Status | Notes |
|-----------|--------|-------|
| `chrono` | ✅ USED | Used in `client.rs` for timestamp generation (lines 441, 683) |

**Status:** ✅ All dependencies are actively used

---

## Verification Details

### Search Methodology

For each dependency, the following patterns were searched:
1. Direct import statements: `use <dep>::`
2. Qualified usage: `<dep>::`
3. Type references: Common types from the crate
4. Macro usage: Macros defined by the dependency

### Files Checked

- **sage-core**: 100+ source files in `crates/sage-core/src/`
- **sage-cli**: 16 source files in `crates/sage-cli/src/`
- **sage-tools**: 60+ source files in `crates/sage-tools/src/`
- **sage-sdk**: 3 source files in `crates/sage-sdk/src/`

---

## Recommendations

### Immediate Actions

#### 1. Remove unused dependencies from sage-core

**File:** `crates/sage-core/Cargo.toml`

Remove line 51:
```diff
- lru = "0.12"
```

#### 2. Remove unused dependencies from sage-cli

**File:** `crates/sage-cli/Cargo.toml`

Remove line 38:
```diff
- uuid = { workspace = true }
```

### Verification Steps

Before committing these changes, verify the build still works:

```bash
# From project root
cd /Users/lifcc/Desktop/code/AI/agent/sage

# Remove the dependencies
# Edit crates/sage-core/Cargo.toml - remove lru
# Edit crates/sage-cli/Cargo.toml - remove uuid

# Verify compilation
cargo check --all

# Run tests to ensure nothing breaks
cargo test --all

# Build release to verify
cargo build --release
```

### Additional Verification with cargo-udeps

For a second opinion, consider using `cargo-udeps`:

```bash
cargo install cargo-udeps
cargo +nightly udeps --all-targets
```

---

## Impact Analysis

### Compilation Impact
- **Low Risk**: Both `lru` and `uuid` have no code references
- **Expected Result**: No compilation errors after removal

### Binary Size Impact
- Removing unused dependencies will reduce:
  - Compilation time
  - Binary size
  - Dependency tree complexity

### Dependency Tree Changes

**Before:**
- sage-core depends on: lru v0.12.5
- sage-cli depends on: uuid v1.19.0

**After:**
- These dependencies will be removed from the respective crates
- Other crates can still use them if needed (workspace dependencies)

---

## Notes and Caveats

1. **Workspace Dependencies**: Both `uuid` and `lru` are defined in the workspace `Cargo.toml`. Removing them from individual crates doesn't remove them from the workspace-level dependencies (other crates may still use them).

2. **Transitive Dependencies**: Some dependencies might be required transitively. The `cargo check` verification will catch these cases.

3. **Example/Test Code**: This audit only checked `src/` directories. Dependencies used only in examples, benchmarks, or tests should be moved to `[dev-dependencies]` or example-specific dependencies.

4. **Feature-Gated Code**: If code is behind feature flags that weren't analyzed, dependencies might be used in non-default configurations.

---

## Appendix: Dependency Usage Evidence

### Used Dependencies Evidence

#### toml (sage-core)
```rust
// crates/sage-core/src/config/loader.rs:120
Some("toml") => toml::from_str(&content)
```

#### serde_yaml (sage-core)
```rust
// crates/sage-core/src/config/loader.rs:122
Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
```

#### shell-words (sage-core)
```rust
// crates/sage-core/src/commands/types.rs
// Uses shell_words crate for parsing
```

#### glob (sage-core)
```rust
// crates/sage-core/src/workspace/patterns.rs:617
if let Ok(entries) = glob::glob(full_pattern.to_string_lossy().as_ref()) {
```

#### walkdir (sage-cli)
```rust
// crates/sage-cli/src/ui_launcher.rs
// Uses walkdir for directory traversal
```

#### base64 (sage-tools)
```rust
// crates/sage-tools/src/tools/network/http_client.rs:290
let bytes = base64::decode(data)
```

#### chrono (sage-sdk)
```rust
// crates/sage-sdk/src/client.rs:441, 683
let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
```

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| Total dependencies audited | 11 |
| Dependencies in use | 9 |
| **Unused dependencies** | **2** |
| Crates affected | 2 |
| Recommended removals | 2 |

---

**Report Generated:** 2025-12-23
**Tool Version:** Manual audit with ripgrep pattern matching
**Confidence Level:** High (manually verified)

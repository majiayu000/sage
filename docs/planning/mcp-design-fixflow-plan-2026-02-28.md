# MCP Design Fixflow Plan (2026-02-28)

Goal:
Unify MCP runtime design, remove redundant/legacy paths, and eliminate tool-registration drift so CLI/SDK behavior is consistent and collision-safe.

Constraints:
- Backward compatibility: not required
- Commit policy: per_step
- Validation scope:
  - Per step: `cargo test --workspace`
  - Final: `cargo test --workspace` + `cargo check --workspace`
- Dirty baseline (pre-existing, not from this run):
  - `crates/sage-core/src/agent/unified/context_builder.rs`
  - `.vibeguard-duplicate-types-allowlist`
  - `FINAL_SUMMARY.txt`
  - `OPTIMIZATION_PROGRESS.md`
  - `OPTIMIZATION_README.md`
  - `OPTIMIZATION_RECOMMENDATIONS.md`
  - `OPTIMIZATION_SUMMARY.md`
  - `OPTIMIZATION_WORK_SUMMARY.md`
  - `docs/optimization/**`
  - `scripts/README.md`
  - `scripts/analyze-clones.sh`
  - `scripts/analyze-unwraps.sh`
  - `scripts/find-optimization-opportunities.sh`
  - `scripts/fix-duplicate-types.sh`
  - `scripts/generate-quality-report.sh`
  - `scripts/run-all-analysis.sh`

Steps:
1. Fix executor initialization drift in SDK/CLI entry points.
   - Scope:
     - Ensure MCP tools are registered before `init_subagent_support` in unified SDK path.
     - Remove duplicated default-provider fallback logic from CLI executor creation paths (use config loader as source of truth).
   - Done condition:
     - SDK unified/run and CLI paths share consistent init ordering semantics.
     - No duplicated default-provider fallback logic remains in CLI entry code.
   - Status: completed

2. Introduce canonical MCP runtime registry in `sage-core` and wire all entry points.
   - Scope:
     - Add a single active MCP registry holder in `sage-core::mcp`.
     - Set/refresh it when MCP registry is built by CLI/SDK paths.
     - Refactor `McpServersTool` to read from canonical core registry.
   - Done condition:
     - `McpServersTool` reflects the same MCP registry used by execution.
   - Status: completed

3. Remove duplicated MCP registry implementation from `sage-tools`.
   - Scope:
     - Replace duplicate registry implementation with thin re-export/alias to `sage-core` types.
     - Remove dead initialization APIs that are no longer needed.
   - Done condition:
     - Only one MCP registry implementation remains (in `sage-core`).
   - Status: completed

4. Namespace MCP tool names to eliminate collisions.
   - Scope:
     - Make MCP adapter-exposed tool names include server namespace.
     - Keep MCP call routing bound to original MCP tool name.
   - Done condition:
     - Multiple servers with same tool name can coexist without override in tool executor.
   - Status: completed

5. Remove KillShell legacy registry path.
   - Scope:
     - Delete legacy `SHELL_REGISTRY` fallback and helper functions.
     - Keep only `BACKGROUND_REGISTRY` path.
     - Update tests accordingly.
   - Done condition:
     - `KillShell` has one registry path, tests pass.
   - Status: completed

6. Final cleanup and verification.
   - Scope:
     - Update MCP docs/comments to match new architecture.
     - Run final full validation matrix.
   - Done condition:
     - All planned steps completed and fully validated.
   - Status: completed

Execution log:
- 2026-02-28: Plan created.
- 2026-02-28 Step 1: executor init/fallback cleanup committed (`3d6ab8d`) and `cargo test --workspace` passed.
- 2026-02-28 Step 2: core active MCP registry + entrypoint wiring committed (`85be23f`) and `cargo test --workspace` passed.
- 2026-02-28 Step 3: duplicate MCP registry implementation removed in `sage-tools` via core aliases (`a1caabd`) and `cargo test --workspace` passed.
- 2026-02-28 Step 4: MCP tool name collision fix with server namespaces (`32e27b3`) and `cargo test --workspace` passed.
- 2026-02-28 Step 5: KillShell legacy registry path removed (`e2883c3`) and demo/example aligned to background registry (`d31821b`); `cargo test --workspace` passed.
- 2026-02-28 Step 6: final validation matrix passed (`cargo test --workspace`, `cargo check --workspace`) and plan/status docs synchronized.

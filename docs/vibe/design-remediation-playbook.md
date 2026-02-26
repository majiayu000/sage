# Vibe Design Remediation Playbook

## Goal
Prevent recurrence of the design issues found in this repository and fix existing violations in a controlled sequence.

## Scope
This playbook covers six classes of issues:
1. Multiple task-management systems with conflicting state.
2. Tool semantics not matching state transitions.
3. Prompt source-of-truth drift.
4. Layering/coupling drift across crates.
5. Version governance drift.
6. Repository boundary hygiene drift.

## Design Principles (Non-Negotiable)
1. Single Source of Truth (SSOT): one canonical state store per domain.
2. Single Responsibility Boundary: `core` stays framework-agnostic.
3. Deterministic Assembly: tool registration must be explicit and non-duplicated.
4. Version Coherence: workspace/internal versions must be aligned.
5. Clean Repository Boundary: no unrelated app artifacts in root.

## Prevention Rules

### Rule A: Task State SSOT
- Canonical task state in agent session scope must be unique.
- Default toolset must not expose parallel task systems that mutate different stores.
- Any completion tool must mutate canonical task state, not only print messages.

### Rule B: Prompt SSOT
- Prompt text consumed at runtime must come from `crates/sage-core/prompts/**`.
- Rust prompt composition layer can orchestrate sections, but section bodies must be loaded from prompt files.
- Guardrails/tests should fail if runtime section and prompt file content drift.

### Rule C: Layering
- `sage-core` must not depend on terminal UI framework crates.
- UI-framework-specific components live in `sage-cli` or optional integration layers.
- `sage-sdk` must be usable without hard dependency on default tool implementation crates.

### Rule D: Versioning
- `workspace.package.version` is the single internal version source.
- Internal path dependencies should not pin stale explicit versions.

### Rule E: Repository Hygiene
- Root-level tracked files must belong to this product.
- Build artifacts (e.g., `*.rlib`) must be ignored and never tracked.

## Step-by-Step Remediation Plan

### Step 1: Documentation and contract
Done condition:
- This playbook exists under `docs/vibe`.
- Each remediation step has verification criteria.

Verification:
- File present and reviewed.

### Step 2: Repository and version governance
Changes:
- Remove unrelated tracked root artifacts.
- Ignore generated binary artifacts.
- Align workspace package version and internal dependency declarations.

Verification:
- `git ls-files` no longer lists unrelated root artifacts.
- `cargo check` passes.

### Step 3: Task-management convergence
Changes:
- Default tool registry exposes only one task-state path.
- `TaskDone` mutates canonical todo state.

Verification:
- Unit tests for task tools pass.
- Tool name uniqueness and schema tests pass.

### Step 4: Prompt SSOT convergence
Changes:
- `system_prompt` section text is sourced from prompt markdown files.
- Build pipeline uses prompt body (without frontmatter) to assemble final prompt.

Verification:
- Prompt unit tests pass.
- Built prompt includes expected section titles/variables.

### Step 5: Layering decoupling
Changes:
- Remove direct `rnk` dependency from `sage-core`.
- Make `sage-sdk -> sage-tools` dependency optional behind default feature.

Verification:
- `cargo check --workspace` passes with defaults.
- `cargo check -p sage-sdk --no-default-features` passes.

### Step 6: Final validation
Minimum matrix:
- Build health: `cargo check --workspace`
- Core architecture guards: `cargo test --package sage-core --test architecture_guards -- --nocapture`
- Tool integration smoke: targeted `sage-tools` tests for task toolset
- Prompt tests: targeted `sage-core` prompt tests

## CI Guardrails to Keep
1. Keep architecture guard tests as hard gate.
2. Add a guard for duplicate task-state systems in default tool registry.
3. Add a guard to ensure prompt sections are loaded from prompt markdown sources.
4. Add a check that internal path dependencies avoid stale pinned versions.
5. Add repository hygiene check for tracked build artifacts (`*.rlib`, etc.).

## Rollback Strategy
- If any step causes regression, rollback only that step commit and keep prior completed steps.
- Never revert unrelated pre-existing dirty files.

## Ownership
- Primary owner: code architecture maintainers.
- Review gate: at least one maintainer review for any change touching `tools/mod.rs`, `prompts/system_prompt.rs`, or crate dependency graph.

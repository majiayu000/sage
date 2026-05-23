# Security and Execution Path Repair Spec (2026-05-23)

Status: ready for implementation

Scope: repair the gap between declared safety/context contracts and the real execution paths used by CLI, SDK, and built-in tools.

Baseline verified during audit:
- `cargo check --workspace --all-targets` passed.
- `cargo test --no-run --message-format=json` passed for the default root package path, but did not exercise all workspace members.

## Problem Statement

Sage has the right high-level split: `sage-core` owns agent/runtime contracts, `sage-tools` owns built-in tools, `sage-cli` owns user entry points, and `sage-sdk` owns library entry points. The current risk is not the crate split itself. The risk is that several safety and context guarantees are declared in one layer but bypassed by the concrete tool execution path.

The repair must close these declaration-execution gaps:

1. Workspace verification does not reliably cover all crates.
2. `working_directory` is resolved by CLI/SDK but not propagated to most default tools.
3. file read-before-write tracking is private to `WriteTool` and not connected to `ReadTool` or `EditTool`.
4. HTTP URL validation happens before the request, but redirects and response saving can bypass the intended SSRF/file-write controls.
5. Bash destructive-command confirmation is represented as a model-supplied `user_confirmed` boolean.
6. command execution security is split across overlapping systems, which makes it easy to patch the wrong layer.
7. docs and Makefile examples contain stale commands that do not match the current repository.

## Non-Goals

- Do not redesign the LLM provider abstraction.
- Do not add new public tool features while repairing safety paths.
- Do not preserve backwards compatibility for unsafe internal tool-call fields such as `user_confirmed`.
- Do not weaken tests, assertions, path checks, or security validation to make implementation easier.
- Do not convert this directly into automation before the manual repair has passed the validation matrix.

## Constraints

- Search before adding new files or abstractions.
- Fix build/verification coverage first before broad refactors.
- Keep each phase independently testable.
- Prefer existing local APIs before inventing new systems.
- No silent degradation: if a safety check cannot validate a path, URL, redirect, or permission, return an error.
- Security-sensitive paths must have regression tests before being marked complete.
- If the same fix fails three times, stop and re-check the architecture instead of continuing patch churn.

## Target Architecture

The intended execution model after the repair:

```text
CLI / SDK entry point
    |
    v
ResolvedExecutionContext
    - working_directory
    - settings
    - input/permission handler
    - tool runtime state
    |
    v
DefaultToolFactory
    |
    +--> file tools share WorkspacePathPolicy + FileAccessTracker
    +--> process tools use the same working_directory and permission path
    +--> network tools use SecureHttpClient + WorkspacePathPolicy for saves
    +--> skill/slash/MCP tools receive the same context
    |
    v
ToolExecutor / orchestrator
    - validate arguments
    - check permission before execute
    - execute with timeout/cancellation
    - record telemetry
```

One resolved context must be the source of truth for working directory, file policy, and permission decisions. Individual tools may expose constructors for tests, but production default registration must not fall back to `std::env::current_dir()` after an explicit working directory has been resolved.

## Phase 0: Make Verification Workspace-Wide

Priority: P0

Root cause:
- root `Cargo.toml` is both a package and a workspace.
- no `default-members` is set.
- `cargo test`, `cargo check`, and `cargo clippy` from the workspace root default to the root package only.
- `Makefile` and CI use commands that look comprehensive but are not consistently workspace-wide.

Files:
- `Cargo.toml`
- `Makefile`
- `.github/workflows/ci.yml`
- `README.md`
- `README_zh.md`
- `CLAUDE.md`

Required changes:
1. Add explicit workspace coverage.
   - Preferred: add `default-members = [".", "crates/sage-core", "crates/sage-cli", "crates/sage-sdk", "crates/sage-tools"]`.
   - Also update scripted commands to use explicit `--workspace` so CI remains clear even if Cargo defaults change.
2. Update Makefile commands:
   - `test`: `cargo test --workspace --all-targets`
   - `check`: `cargo check --workspace --all-targets --all-features`
   - `clippy`: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - keep narrower package/test shortcuts only if their names say they are narrow.
3. Update CI:
   - check job must use `cargo check --workspace --all-targets --all-features`.
   - clippy job must use `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
   - test job should use `cargo test --workspace --all-targets --verbose` unless release timing requires a documented narrower matrix.
4. Update docs that claim `cargo test` is full workspace coverage.

Acceptance tests:
- `cargo metadata --no-deps --format-version 1` shows all intended workspace packages in `workspace_default_members`.
- `make check` checks all crates.
- `make test` compiles and runs all workspace tests.
- `make clippy` lints all crates.
- CI yaml contains no root-only `cargo check`, root-only `cargo test`, or root-only `cargo clippy` for the main quality gates.

Done when:
- `sage-cli` is exercised by local and CI quality gates without requiring a special command.

## Phase 1: Propagate Working Directory Into All Default Tools

Priority: P0

Root cause:
- `get_default_tools_with_context()` receives `working_directory`, but `build_default_tools()` creates most tools with `new()`.
- file, process, and code-intelligence tools therefore use process cwd instead of the resolved CLI/SDK working directory.
- SDK `default_tools()` calls `sage_tools::get_default_tools()` and loses the SDK working directory entirely.

Files:
- `crates/sage-tools/src/tools/mod.rs`
- `crates/sage-tools/src/tools/file_ops/edit.rs`
- `crates/sage-tools/src/tools/file_ops/read/tool.rs`
- `crates/sage-tools/src/tools/file_ops/write/types.rs`
- `crates/sage-tools/src/tools/file_ops/glob/types.rs`
- `crates/sage-tools/src/tools/file_ops/grep/mod.rs`
- `crates/sage-tools/src/tools/file_ops/notebook_edit/mod.rs`
- `crates/sage-tools/src/tools/process/bash/types.rs`
- `crates/sage-tools/src/tools/code_intelligence/lsp/mod.rs`
- `crates/sage-sdk/src/client/execution/mod.rs`
- `crates/sage-sdk/src/client/execution/run.rs`
- `crates/sage-sdk/src/client/execution/unified.rs`
- `crates/sage-cli/src/commands/unified/execute.rs`
- `crates/sage-cli/src/ui/rnk_app/executor/creation.rs`

Required changes:
1. Introduce a production default-tool config, for example:

```rust
pub struct DefaultToolConfig {
    pub working_directory: PathBuf,
    pub skill_registry: Arc<RwLock<SkillRegistry>>,
    pub file_access_tracker: Arc<FileAccessTracker>,
}
```

2. Replace `build_default_tools(skill_tool, slash_command_tool)` with a context-aware factory.
3. Register context-bound tool instances:
   - `EditTool::with_working_directory(...)`
   - `ReadTool::with_working_directory(...)`
   - `WriteTool::with_working_directory(...)`
   - `GlobTool::with_working_directory(...)`
   - `GrepTool::with_working_directory(...)`
   - `NotebookEditTool::with_working_directory(...)`
   - `BashTool::with_working_directory(...)`
   - `LspTool::with_working_directory(...)`
4. Keep `new()` constructors for tests and simple direct use, but production CLI/SDK paths must call the context-aware factory.
5. Update SDK default tool creation so `.with_working_directory(...)` and run/unified options affect built-in tools.

Acceptance tests:
- Add an integration test that sets process cwd to directory A, creates default tools with working directory B, and proves:
  - Bash `pwd` runs in B.
  - Read/Write/Glob/Grep operate relative to B.
  - LSP workspace root is B when the tool is available.
- Add SDK tests proving `SageClient::with_working_directory(B)` does not use process cwd for default tools.
- Search-based guard: production default factory must not instantiate context-sensitive tools with `new()`.

Done when:
- an explicit working directory affects every built-in tool that reads, writes, searches, executes, or analyzes local files.

## Phase 2: Replace Per-Tool Read Tracking With Shared File Access State

Priority: P0

Root cause:
- `WriteTool` owns a private `read_files` set.
- `ReadTool` cannot mark paths in that set.
- `EditTool` does not enforce the read-before-edit contract.
- tool execution through the normal registry therefore cannot reliably distinguish inspected files from blind overwrites.

Files:
- `crates/sage-tools/src/tools/file_ops/read/**`
- `crates/sage-tools/src/tools/file_ops/write/**`
- `crates/sage-tools/src/tools/file_ops/edit.rs`
- `crates/sage-tools/src/tools/file_ops/notebook_edit/**`
- `crates/sage-tools/src/tools/mod.rs`
- tests under `crates/sage-tools/tests/` or tool-local test modules

Required design:
1. Add a shared `FileAccessTracker`.

```rust
pub struct FileAccessTracker {
    read_files: RwLock<HashSet<PathBuf>>,
}

impl FileAccessTracker {
    pub async fn mark_read(&self, canonical_path: PathBuf);
    pub async fn has_read(&self, canonical_path: &Path) -> bool;
    pub async fn clear(&self);
}
```

2. Store canonical workspace-safe paths, not raw user input.
3. Inject the same tracker into Read, Write, Edit, and NotebookEdit through the default tool factory.
4. `ReadTool` marks a file as read only after a successful read.
5. `WriteTool` and `EditTool` require a prior read for existing files.
6. New-file creation does not require a prior read, but still requires path-policy approval.
7. Remove or deprecate `WriteTool`'s private tracker constructors so new production code cannot accidentally create isolated state.

Acceptance tests:
- Existing file write without prior read is denied.
- Read existing file, then write it through a separately registered WriteTool instance sharing the same tracker, succeeds.
- Read existing file, then edit it, succeeds.
- Edit existing file without prior read is denied.
- New file creation succeeds inside workspace.
- Tracker stores canonical paths, so equivalent relative paths do not bypass the rule.

Done when:
- read-before-write/edit is enforced by shared execution state, not by a private field in one tool.

## Phase 3: Centralize Workspace Path Policy

Priority: P0

Root cause:
- file tools have partial path checks.
- HTTP `save_to_file` writes directly through `tokio::fs::write`.
- path validation for new files and non-existent parents is inconsistent.

Files:
- `crates/sage-tools/src/tools/file_ops/**`
- `crates/sage-tools/src/tools/network/http_client/request.rs`
- `crates/sage-tools/src/tools/network/http_client/types.rs`
- possible new module: `crates/sage-tools/src/tools/file_ops/policy.rs`

Required design:
1. Add a central `WorkspacePathPolicy` used by every tool that reads or writes local files.
2. Policy inputs:
   - `working_directory`
   - user-supplied path
   - operation kind: read, create, overwrite, edit, save response
3. Policy output:
   - canonical or safely resolved absolute path
   - relative display path for user-facing messages
4. Rules:
   - relative paths resolve under `working_directory`.
   - absolute paths are allowed only if they canonicalize inside `working_directory`.
   - `..` cannot escape workspace.
   - symlinked parents cannot escape workspace.
   - if validation cannot determine safety, deny.
   - existing-file overwrite/edit must also pass `FileAccessTracker`.
5. Route `http_client.save_to_file` through the same policy and write service.
6. If the policy makes `save_to_file` too complex, remove `save_to_file` from the public schema and require callers to fetch first, then write through the file tools.

Acceptance tests:
- `../outside.txt` is denied.
- absolute path outside workspace is denied.
- symlink inside workspace pointing outside workspace is denied for read/write.
- non-existent file inside workspace is allowed for create.
- existing file inside workspace requires prior read for overwrite/edit.
- `http_client.save_to_file` cannot write outside workspace.
- `http_client.save_to_file` cannot overwrite an existing file without satisfying the same overwrite policy.

Done when:
- no built-in tool writes local files without passing through the same workspace path policy.

## Phase 4: Make HTTP Fetching Redirect-Safe and Save-Safe

Priority: P0

Root cause:
- URL validation currently happens before request execution.
- reqwest redirect handling can follow to a private, loopback, link-local, or metadata URL after the initial URL passes validation.
- DNS validation and request resolution are separate events.
- WebFetch and HttpClient implement similar security logic separately.

Files:
- `crates/sage-tools/src/tools/network/validation.rs`
- `crates/sage-tools/src/tools/network/http_client/validation.rs`
- `crates/sage-tools/src/tools/network/http_client/request.rs`
- `crates/sage-tools/src/tools/network/web_fetch.rs`
- possible new module: `crates/sage-tools/src/tools/network/secure_client.rs`

Required design:
1. Create one shared secure network module for WebFetch and HttpClient.
2. Validate:
   - scheme is `http` or `https`.
   - host is present.
   - host is not localhost, loopback, private, link-local, multicast, unspecified, or metadata IP.
   - resolved target addresses are public.
3. Disable reqwest's implicit redirect following in tool-specific clients.
4. Implement explicit redirect handling:
   - maximum 10 redirects.
   - validate each redirect target before following.
   - resolve relative redirect locations against the current URL.
   - reject redirect target if validation fails.
   - avoid forwarding sensitive headers across origin changes unless explicitly safe.
5. After response, validate final URL and remote address when the HTTP client exposes it.
6. Use the central `WorkspacePathPolicy` for response saving.

Acceptance tests:
- initial localhost/private/metadata URLs are denied.
- public-looking URL redirecting to localhost/private/metadata is denied.
- redirect chain exceeding limit is denied.
- relative redirects are resolved and validated.
- WebFetch and HttpClient share the same redirect validation behavior.
- `save_to_file` obeys the file policy from Phase 3.

Done when:
- there is no direct security-sensitive request path that validates only the initial URL and then lets reqwest follow redirects automatically.

## Phase 5: Move Bash Destructive Confirmation Into Core Permission Flow

Priority: P0

Root cause:
- Bash schema exposes `user_confirmed`.
- the tool trusts this model-supplied bool to allow destructive commands.
- the real permission UI path currently retries by injecting `user_confirmed = true`, which means direct callers can bypass it.
- `ToolExecutor` has a permission API surface, but the basic executor path does not enforce it before execution.

Files:
- `crates/sage-tools/src/tools/process/bash/mod.rs`
- `crates/sage-tools/src/tools/process/bash/validation.rs`
- `crates/sage-core/src/tools/executor.rs`
- `crates/sage-core/src/tools/base/tool_trait.rs`
- `crates/sage-core/src/tools/permission/**`
- `crates/sage-core/src/agent/unified/step_execution.rs`
- `crates/sage-tools/tests/bash_tool_integration.rs`

Required design:
1. Remove `user_confirmed` from Bash public schema.
2. Implement `BashTool::check_permission(...)`.
   - safe commands return `ToolPermissionResult::Allow`.
   - destructive commands return `ToolPermissionResult::Ask` or `Deny`, depending on policy/settings/context.
   - critical destructive commands can return `Deny` regardless of model request.
3. Ensure the real tool execution path checks permission before `execute`.
   - Either extend `ToolExecutor` with permission handler/context support.
   - Or ensure CLI/SDK never use a path that bypasses `ParallelToolExecutor` permission checks.
4. Delete the retry path that catches `ConfirmationRequired` and injects `user_confirmed`.
5. Non-interactive execution without an explicit allow policy must deny high-risk destructive commands by default.
6. Permission-granted responses should be represented by a handler decision, not by modifying the model-supplied arguments.

Acceptance tests:
- Bash schema contains no `user_confirmed`.
- direct Bash call with `user_confirmed: true` does not bypass permission.
- destructive command without permission handler is denied.
- destructive command with deny handler is denied.
- destructive command with allow handler executes.
- safe command executes without prompting.
- unified agent path prompts through `InputRequest::permission` and executes only after permission is granted.

Done when:
- no user confirmation decision is represented by a field the model can set directly.

## Phase 6: Collapse Command Execution Onto One Runtime Contract

Priority: P1

Root cause:
- command security is currently split between BashTool validation, core sandbox modules, permission modules, hooks, and executor/orchestrator paths.
- docs claim several controls exist, but the real path for Bash is still direct `bash -c` from `sage-tools`.

Files:
- `crates/sage-core/src/sandbox/**`
- `crates/sage-core/src/tools/executor.rs`
- `crates/sage-core/src/tools/parallel_executor/**`
- `crates/sage-tools/src/tools/process/bash/**`
- `docs/architecture/command-execution-security.md`

Required design:
1. Define a single command execution service contract:

```rust
pub struct CommandExecutionRequest {
    pub command: String,
    pub working_directory: PathBuf,
    pub timeout: Duration,
    pub environment: HashMap<String, String>,
    pub risk: RiskLevel,
}
```

2. The service must perform, in order:
   - command validation
   - permission check
   - hook/preflight check if configured
   - sandbox/resource policy if enabled
   - execution
   - post-execution telemetry/audit
3. BashTool should delegate to this service instead of owning a parallel execution model.
4. If a short-term full unification is too large, document the remaining split with an explicit TODO and a test proving the active path still enforces Phases 1 and 5.

Acceptance tests:
- Bash uses the same working directory source as file tools.
- Bash permission checks run before process spawn.
- timeout/cancellation behavior is preserved.
- existing background shell behavior remains tested.
- docs no longer claim inactive sandbox paths protect active Bash execution.

Done when:
- a security fix in the central command path necessarily affects the Bash tool path.

## Phase 7: Align Docs, Make Targets, and Examples

Priority: P2

Root cause:
- `Makefile` examples reference missing example files.
- docs overstate current security behavior.
- several docs refer to root-only cargo commands as if they were full workspace checks.

Files:
- `Makefile`
- `README.md`
- `README_zh.md`
- `CLAUDE.md`
- `docs/architecture/command-execution-security.md`
- `docs/tools/2-2/bash.md`
- `docs/tools/2-2/bash_zh.md`
- docs that mention `cargo test`, `cargo check`, or missing examples

Required changes:
1. Replace missing example targets:
   - remove `markdown_demo` and `ui_demo`, or replace with existing examples such as `read_tool_demo`, `grep_demo`, and `planning_demo`.
2. Update security docs after Phases 4-6.
3. Update command examples to use workspace-wide commands where appropriate.
4. Document narrow commands as narrow, for example `cargo test -p sage-tools`.

Acceptance tests:
- `make examples` succeeds or only references examples that exist.
- docs do not claim inactive security layers protect active execution.
- docs distinguish workspace-wide commands from package-local commands.

Done when:
- a new contributor following README/Makefile gets the same coverage as CI.

## Required Regression Test Inventory

Add or update tests for these cases before marking the repair complete:

| Area | Test case |
| --- | --- |
| Workspace gates | default members include root + all four crates |
| Workspace gates | Makefile/CI commands use `--workspace` for main gates |
| Working directory | default tools use explicit working directory, not process cwd |
| SDK working directory | SDK `.with_working_directory` affects built-in tools |
| File tracker | read then write through separate registered tools succeeds |
| File tracker | write/edit existing file without read is denied |
| Path policy | `..`, absolute outside, and symlink escape are denied |
| HTTP | redirect to private/metadata/loopback target is denied |
| HTTP | `save_to_file` obeys workspace policy |
| Bash permission | schema has no `user_confirmed` |
| Bash permission | destructive command cannot self-confirm through arguments |
| Bash permission | permission handler allow/deny decisions control execution |
| Docs/examples | `make examples` references existing examples only |

## Final Validation Matrix

Run these after all phases:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
make examples
```

For security-specific changes, also run targeted tests:

```bash
cargo test -p sage-tools file_ops
cargo test -p sage-tools network
cargo test -p sage-tools bash
cargo test -p sage-core permission
cargo test -p sage-core --test architecture_guards -- --nocapture
```

## Rollout Plan

Recommended implementation sequence:

1. Phase 0 only, then run the workspace validation gate.
2. Phase 1, then run working-directory integration tests plus workspace check.
3. Phase 2 and Phase 3 together only if the shared tracker and path policy API are small; otherwise split them.
4. Phase 4, then run network tests and workspace check.
5. Phase 5, then run permission and Bash tests.
6. Phase 6 as a follow-up refactor after Phases 0-5 are green.
7. Phase 7 last, after behavior is final.

Do not start Phase 6 before Phases 0-5 pass. The first five phases close concrete safety bugs; Phase 6 is architectural consolidation.

## Completion Definition

The repair is complete when:

- full workspace verification is the default local and CI path.
- all context-sensitive default tools receive the resolved working directory.
- file write/edit behavior cannot bypass shared read tracking and workspace policy.
- HTTP fetch/save behavior cannot bypass SSRF and workspace file-write checks through redirects or direct saves.
- Bash destructive commands require a real permission decision outside model-controlled arguments.
- command-execution security docs describe the active runtime path, not an aspirational design.
- the final validation matrix passes in a fresh run.

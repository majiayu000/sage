# Tech Spec

## Linked Issue

GH-87

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| MCP config | `crates/sage-core/src/config/mcp_config.rs` | Direct MCP configuration is parsed separately | Source merge starts here |
| MCP discovery | `crates/sage-core/src/mcp/discovery/**` | Discovers and connects to MCP servers | Needs deferred discovery and structured status |
| MCP registry | `crates/sage-core/src/mcp/{registry,runtime_registry,error}.rs` | Tracks runtime MCP server/tool state | Natural place for status and source metadata |
| MCP transports | `crates/sage-core/src/mcp/transport/**` | Handles server process/transport connection | Controlled startup and fail-closed transport rules live here |
| MCP tools | `crates/sage-tools/src/mcp_tools/**` | Exposes MCP tools to tool callers | Needs list/search/status and structured execution errors |
| Extension packages | `specs/GH86/**` | Planned source for package-declared MCP servers | GH-87 should consume its source metadata contract |

## 设计方案

Future implementation should add a runtime source model before changing execution:

- `crates/sage-core/src/mcp/source.rs`
- `crates/sage-core/src/mcp/runtime_status.rs`
- `crates/sage-core/src/mcp/auth_status.rs`
- `crates/sage-core/src/mcp/deferred_tools.rs`

The source model should accept direct config sources now and package sources once GH-86 lands. Runtime status should be inspectable without needing every server to be connected.

## Source Model Sketch

Fields:

- `server_id`
- `source_kind`
- `source_ref`
- `enabled`
- `config_hash`
- `package_id`
- `auth_state`
- `last_connect_attempt`
- `last_error`
- `tool_discovery_state`

Source precedence should be encoded in one merge function and covered with fixtures. Duplicate sources should not be resolved by map insertion order.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Source metadata | source merge | direct/package fixture tests |
| Auth status | auth status model | auth pending/recovery tests |
| Controlled startup | runtime registry | connect/disconnect/retry tests |
| Deferred tools | deferred tool index | list/search freshness tests |
| Structured failures | MCP error types | connection/auth/schema tests |

## 数据流

1. Config loader produces direct MCP server declarations.
2. Package bridge from GH-86 produces package-sourced declarations.
3. Source merge validates enabled sources and precedence.
4. Runtime registry exposes server status without eager tool loading.
5. Deferred discovery indexes tools as servers connect or refresh.
6. Tool execution checks auth/connection/schema state before invoking transport.

## 备选方案

- Eager-connect every MCP server at startup: rejected because failures and slow servers would block unrelated runtime use.
- Ignore disabled or failing package servers silently: rejected because enabled-source failures must be diagnosable.
- Store auth state only in logs: rejected because callers need programmatic recovery status.

## 风险

- Security: remote stdio or uncontrolled transport can bypass process controls.
- Reliability: partial MCP failure must not break unrelated local tools.
- UX: auth_required and schema_error must be actionable.
- Ordering: direct vs package precedence must be deterministic.

## 测试计划

- Source merge fixture tests for direct and package declarations.
- Auth pending/recovery status tests.
- Controlled connect/disconnect/retry tests with fake transports.
- Deferred tool list/search tests.
- Structured connection/auth/schema error tests.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep existing direct MCP config execution path behind a compatibility adapter. If runtime source merge fails, disable only the affected MCP source and report structured status while keeping non-MCP tools available.

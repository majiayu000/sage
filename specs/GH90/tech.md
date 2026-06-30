# Tech Spec

## Linked Issue

GH-90

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Telemetry | `crates/sage-core/src/telemetry/**` | Captures tool/provider events | Needs bounded ring storage and redaction |
| Settings | `crates/sage-core/src/settings/**` | Loads and validates settings | Managed config and source provenance belong here |
| Permission policy | `crates/sage-core/src/agent/unified/settings_permission.rs`, `crates/sage-core/src/tools/permission/handlers/policy.rs` | Computes and explains permissions | Audit needs policy source and reason |
| Sandbox violations | `crates/sage-core/src/sandbox/violations/**` | Records sandbox violation information | Diagnostic bundle should include redacted sandbox summary |
| Storage schema | `crates/sage-core/src/storage/schema.rs` | Defines durable storage schema | Optional audit/ring persistence may need schema coordination |
| Provider errors | `crates/sage-core/src/llm/providers/error_utils.rs` | Redacts provider errors | Should feed bundle redaction and recovery hints |
| Diagnostics CLI | `crates/sage-cli/src/commands/diagnostics/**` | User-facing diagnostics entry point | Natural user consent and bundle command surface |

## 设计方案

Future implementation should add a diagnostics domain that consumes existing telemetry/settings/provider/sandbox signals:

- `crates/sage-core/src/diagnostics/event_ring.rs`
- `crates/sage-core/src/diagnostics/redaction.rs`
- `crates/sage-core/src/diagnostics/bundle.rs`
- `crates/sage-core/src/settings/managed_config.rs`
- `crates/sage-core/src/audit/policy_source.rs`

The bundle generator should be pure over collected snapshots so tests can run without real user data or network access.

## Diagnostic Event Sketch

Fields:

- `event_id`
- `timestamp`
- `kind`
- `thread_id`
- `source`
- `severity`
- `redaction_class`
- `payload_summary`
- `dropped_count`

Events should store summaries by default, not raw command output or secret-bearing payloads.

## Bundle Sections

- doctor summary
- config source stack
- provider/model/auth status summary
- proxy/network summary
- sandbox/permission denial summary
- recent diagnostic events
- audit source/provenance summary
- redaction report

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Bounded capture | event ring | capacity/dropped-count tests |
| Redacted bundle | bundle/redaction | secret fixture tests |
| User consent | diagnostics CLI | consent/decline tests |
| Managed strict config | settings managed config | unknown-field/precedence tests |
| Policy provenance | audit/policy source | denial reason tests |

## 数据流

1. Runtime emits diagnostic events into bounded ring.
2. Settings loader records config source and managed policy provenance.
3. Permission/sandbox/provider decisions emit redacted audit summaries.
4. User requests diagnostics bundle and explicitly consents.
5. Bundle builder snapshots event ring and source summaries.
6. Redaction runs before writing or uploading any bundle artifact.

## 备选方案

- Dump raw logs into feedback bundle: rejected because it risks leaking secrets.
- Use unbounded telemetry Vec: rejected because long sessions need bounded memory.
- Let managed config override safety denies: rejected because managed config must not weaken higher-priority safety.

## 风险

- Security: redaction gaps can leak credentials or file paths.
- Reliability: audit logging must not crash the main runtime.
- UX: overly redacted bundles may become useless unless redaction report explains removals.
- Data retention: bundle and audit retention must be explicit.

## 测试计划

- Ring capacity and dropped-count tests.
- Redaction fixture tests for tokens, credentials, cookies and provider keys.
- Feedback bundle consent/decline tests.
- Managed config strict schema and precedence tests.
- Policy source/audit reason tests.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep diagnostics bundle generation gated behind explicit consent. If managed config parsing or audit capture fails, return structured diagnostics errors and keep runtime safety decisions unchanged.

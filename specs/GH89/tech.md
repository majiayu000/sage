# Tech Spec

## Linked Issue

GH-89

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Credential config | `crates/sage-core/src/config/credential/**` | Resolves credential sources and tests | Natural home for backend trait and precedence |
| Provider registry | `crates/sage-core/src/config/provider_registry.rs` | Manages provider definitions | Needs catalog merge/freshness metadata |
| Models API | `crates/sage-core/src/config/models_api.rs` | Fetches or describes model metadata | Needs TTL/ETag cache behavior |
| Embedded providers | `crates/sage-core/src/config/embedded_providers.rs` | Static fallback provider data | Offline fallback source |
| Provider config | `crates/sage-core/src/config/provider/**` | Provider-specific configuration | Needs credential source and catalog status wiring |
| Model capabilities | `crates/sage-core/src/llm/model_capabilities.rs` | Capability decisions exist separately | Should become the single capability manager |
| Provider errors | `crates/sage-core/src/llm/providers/error_utils.rs` | Produces provider error hints/redaction | Needs credential recovery and revoke hints |

## 设计方案

Future implementation should separate credential persistence from provider/model catalog:

- `crates/sage-core/src/config/credential/backend.rs`
- `crates/sage-core/src/config/credential/source_precedence.rs`
- `crates/sage-core/src/config/model_catalog.rs`
- `crates/sage-core/src/llm/capability_manager.rs`

Credential resolution should be deterministic and auditable. Catalog refresh should never require real network access in unit tests.

## Credential Source Precedence

Expected order:

1. Explicit runtime/session credential input.
2. Environment variables.
3. Secure credential backend.
4. Legacy imported plaintext config when explicitly allowed.
5. Missing credential structured error.

Every resolved credential should include provider id, source kind and redacted display metadata.

## Catalog Cache Sketch

Fields:

- `provider_id`
- `models`
- `capabilities`
- `etag`
- `fetched_at`
- `ttl`
- `freshness`
- `source`
- `last_error`

Catalog lookup should merge remote cache with embedded static data using one deterministic function.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Source precedence | credential resolver | env/saved/legacy fixture tests |
| Secure backend | credential backend | fake backend save/logout tests |
| Revoke recovery | credential operations | revoke failure tests |
| Catalog freshness | model catalog | TTL/ETag/offline tests |
| Single capability manager | capability manager | unknown model fallback tests |

## 数据流

1. Provider request asks credential resolver for a redacted credential handle.
2. Resolver checks source precedence and returns source metadata.
3. Provider/model catalog loads cache or embedded fallback.
4. Refresh path uses ETag/TTL and records freshness state.
5. Capability manager answers feature support from merged catalog.
6. Provider errors include credential/catalog recovery hints without exposing secrets.

## 备选方案

- Continue storing credentials only in JSON config: rejected because durable secret storage should prefer platform backends.
- Fetch model catalog on every startup: rejected because offline and latency behavior must be predictable.
- Keep per-provider capability fallback logic: rejected because inconsistent unknown-model behavior causes drift.

## 风险

- Security: accidental plaintext secret persistence or log exposure.
- Compatibility: legacy config migration must not break existing users.
- Reliability: remote catalog outage must not remove known static capabilities.
- Maintainability: duplicate capability logic can reappear if manager is not the only API.

## 测试计划

- Credential source precedence fixture tests.
- Fake secure backend save/logout/revoke tests.
- Legacy plaintext import/fallback tests with fake data.
- Catalog TTL/ETag/offline fallback tests with mock HTTP.
- Capability manager unknown-model fallback tests.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep legacy credential resolver as a compatibility fallback behind the new resolver. If secure backend write fails, return a structured unsupported/error state and do not silently downgrade to plaintext persistence.

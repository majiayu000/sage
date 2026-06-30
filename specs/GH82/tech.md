# Tech Spec

## Linked Issue

GH-82

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Sessions | `crates/sage-core/src/session/**` | Session metadata, cache, storage and JSONL helpers exist | Primary source for thread metadata and restart recovery |
| Trajectory | `crates/sage-core/src/trajectory/**` | Records JSONL entries for session start, messages, tool calls, errors and session end | Backfill source for thread/turn/item indexes |
| Storage | `crates/sage-core/src/storage/**` | SQLite/backend abstractions already exist | Candidate home for migrations and query APIs |
| Protocol | `specs/GH81/**` | Defines `thread_id`, `turn_id`, `item_id`, request/notification/error envelope | ThreadStore must persist the same identity model |
| Runtime | `crates/sage-core/src/agent/unified/**` | Unified executor owns session execution and recorder setup | Future integration point after store contract lands |

## 设计方案

Future implementation should add a ThreadStore module under `sage-core`, likely near the existing session/storage boundary:

- `crates/sage-core/src/thread_store/mod.rs`
- `crates/sage-core/src/thread_store/types.rs`
- `crates/sage-core/src/thread_store/trait.rs`
- `crates/sage-core/src/thread_store/sqlite.rs`
- `crates/sage-core/src/thread_store/backfill.rs`
- `crates/sage-core/src/thread_store/migrations/**`

The store should persist metadata/index rows in SQLite while keeping large legacy JSONL payloads referenced by path until a later migration explicitly moves payloads.

## Contract Sketch

`ThreadStore` should cover:

- `create_thread(metadata) -> ThreadRecord`
- `resume_thread(thread_id) -> ThreadSnapshot`
- `append_event(thread_id, turn_id, item) -> AppendResult`
- `flush(thread_id) -> StoreResult`
- `read_thread(thread_id, options) -> ThreadSnapshot`
- `list_threads(query, pagination) -> Page<ThreadSummary>`
- `search_threads(query, pagination) -> Page<SearchHit>`
- `archive_thread(thread_id, reason) -> ThreadRecord`
- `unarchive_thread(thread_id) -> ThreadRecord`
- `delete_thread(thread_id, deletion_mode) -> DeleteResult`
- `backfill_legacy(source_path, options) -> BackfillReport`

## Schema Sketch

Minimum SQLite tables:

- `thread_store_schema(version, applied_at)`
- `threads(thread_id, legacy_session_id, title, cwd, provider, model, status, archived_at, created_at, updated_at)`
- `turns(turn_id, thread_id, status, started_at, completed_at, sequence_start, sequence_end)`
- `items(item_id, thread_id, turn_id, item_type, role, status, source, created_at, sequence, legacy_uuid, payload_ref)`
- `thread_lineage(thread_id, parent_thread_id, parent_turn_id, parent_item_id, fork_mode)`
- `legacy_sources(source_id, path, source_kind, imported_at, checksum, import_status)`
- `store_errors(error_id, thread_id, source_id, code, message, details, created_at)`

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Stable IDs and lineage | `thread_store/types.rs`, migrations | ID/lineage unit tests |
| JSONL backfill compatibility | `thread_store/backfill.rs` | fixture import tests with legacy trajectory entries |
| Query/list/search/archive | SQLite query layer | pagination/search/archive tests |
| Restart recovery | store bootstrap + runtime integration seam | interrupted turn and restart tests |
| No silent data loss | structured store errors | corrupt JSONL and SQLite failure tests |

## 数据流

1. Runtime emits GH-81 protocol items.
2. ThreadStore appends metadata/index rows and references any large payload.
3. Backfill scans legacy JSONL/session files and creates thread/turn/item index rows.
4. CLI/SDK/runtime facade can list/search/read through the store in GH-83.
5. Child-agent graph can store parent/child edges through this identity model in GH-84.

## 备选方案

- Keep only JSONL scanning: rejected because list/search/archive/recovery need indexed metadata and stable pagination.
- Move all payloads into SQLite immediately: deferred to reduce migration risk and preserve existing JSONL compatibility.
- Add ThreadStore directly to CLI: rejected because store belongs in core runtime state.

## 风险

- Migration risk: backfill must be idempotent and versioned.
- Corrupt legacy data: importer needs partial success reporting and structured errors.
- Concurrency: append/flush must not corrupt indexes when tools or child agents emit events concurrently.
- Privacy: search index must avoid indexing secrets unless redaction policy is defined.

## 测试计划

- Unit tests for ThreadStore trait contracts and error variants.
- SQLite migration tests with empty, current and old schema versions.
- Backfill fixture tests from trajectory/session JSONL.
- Query tests for pagination, search, archive/unarchive and delete modes.
- Restart recovery tests for incomplete turns and interrupted writes.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep legacy JSONL as the compatibility fallback. If a migration fails after implementation, disable ThreadStore startup, preserve the old JSONL files, and surface a structured repair command rather than deleting generated indexes.

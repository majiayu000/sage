# Task Plan

## Linked Issue

GH-82

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP82-T01` Owner: state. Done when: ThreadStore trait/types define create, resume, append, flush, read, list, search, archive, unarchive, delete and backfill contracts. Verify: trait/type compile tests pass.
- [ ] `SP82-T02` Owner: state. Done when: SQLite migrations create schema version, threads, turns, items, lineage, legacy sources and store error tables. Verify: migration tests pass from empty and existing DB states.
- [ ] `SP82-T03` Owner: state. Done when: legacy JSONL/session/trajectory backfill imports recoverable records with source references and structured partial-error reports. Verify: backfill fixture tests pass.
- [ ] `SP82-T04` Owner: state. Done when: list/read/search/archive/unarchive/delete queries have stable pagination and sorting. Verify: query and archive/delete tests pass.
- [ ] `SP82-T05` Owner: state. Done when: restart recovery detects incomplete turns, schema mismatch, corrupt JSONL and read-only DB failures without silent empty-state fallback. Verify: recovery and failure tests pass.
- [ ] `SP82-T06` Owner: coordinator. Done when: this focused spec PR links GH-82, keeps cloud sync/App/IDE/app-server-client out of scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

GH-82 should start after GH-81 protocol types are accepted. Within GH-82, migrations and backfill can be implemented in separate branches only after the trait/type contract is stable. Query/archive/delete should wait for the migration schema.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH82`
- Forbidden typo scan over `specs/GH82`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core thread_store`
- `cargo test -p sage-core thread_store_backfill`
- `cargo test -p sage-core thread_store_migration`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #82` for spec-only PRs. Use a closing keyword only on the final implementation PR that satisfies the ThreadStore acceptance criteria.

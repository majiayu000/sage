# Tech Spec

## Linked Issue

GH-86

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Plugin package core | `crates/sage-core/src/plugins/{manifest,registry,lifecycle}.rs` | Existing plugin concepts are split across manifest, registry and lifecycle code | Natural home for extension package schema and lifecycle APIs |
| Skills | `crates/sage-core/src/skills/registry/**`, `crates/sage-core/src/skills/types/**` | Skills are discoverable/registerable assets | Packages must register and unregister skills safely |
| Hooks | `crates/sage-core/src/hooks/**` | Hooks can alter execution behavior | Package hook declarations need strict enable/disable control |
| Commands | `crates/sage-core/src/commands/registry/**` | Commands are registry-backed | Packages must avoid command collisions and stale entries |
| MCP config | `crates/sage-core/src/config/mcp_config.rs` | MCP servers are configured separately | GH-86 should provide source metadata for GH-87 to consume |
| Locations | `crates/sage-core/src/settings/locations.rs` | Defines user/project config paths | Package install roots and path boundaries should use existing locations |

## 设计方案

Future implementation should introduce a versioned extension package layer around the existing plugin modules:

- `crates/sage-core/src/plugins/package_manifest.rs`
- `crates/sage-core/src/plugins/package_store.rs`
- `crates/sage-core/src/plugins/package_lifecycle.rs`
- `crates/sage-core/src/plugins/package_registry_bridge.rs`

The package layer should keep manifest parsing, storage state and registry mutation separate. Registry mutation should be transactional where possible: validate all declared assets before mutating any registry.

## Manifest Schema Sketch

Top-level fields:

- `schema_version`
- `id`
- `name`
- `version`
- `description`
- `assets.skills`
- `assets.mcp_servers`
- `assets.hooks`
- `assets.commands`
- `dependencies`
- `permissions`
- `metadata`

Every asset entry should include a relative path or inline declaration, source metadata and an enabled-state dependency on the package.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Versioned strict manifest | package manifest parser | valid/invalid fixture tests |
| Package root boundary | package store | path escape fixture tests |
| Disabled packages do not register | lifecycle coordinator | disabled-state lifecycle tests |
| Reversible registration | registry bridge | enable-disable-uninstall tests |
| Conflict fail closed | registry bridge | duplicate command/skill tests |

## 数据流

1. Discover scans allowed package roots and reads manifest candidates.
2. Parser validates schema, paths, dependencies and permission declarations.
3. Install copies or records package assets under the managed package store.
4. Enable validates the installed package and registers declared assets.
5. Disable unregisters declared assets while preserving installed package state.
6. Uninstall disables first, then removes package metadata and assets.

## 备选方案

- Keep per-system installers: rejected because skills/MCP/hooks/commands need consistent package provenance.
- Trust manifest paths without root checks: rejected due path traversal risk.
- Register assets at install time only: rejected because users need disabled installed packages.

## 风险

- Security: package path traversal or undeclared hook execution.
- Registry integrity: partial enable/disable can leave stale entries.
- Compatibility: existing skill/plugin discovery must continue to work.
- UX: error messages must identify package id, asset id and failing path.

## 测试计划

- Manifest parser fixture tests.
- Package lifecycle tests for install, enable, disable, uninstall.
- Path escape and missing dependency tests.
- Registry conflict tests for skill/command/hook/MCP declarations.
- Disabled package tests proving no asset is visible in runtime registries.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep existing direct skill/MCP/hook/command registration paths while package support is gated. If package lifecycle fails, disable the affected package and leave direct registries unchanged.

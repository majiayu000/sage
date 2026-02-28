# Sage Agent - Quick Action Checklist

> Priority-ordered action items extracted from comprehensive analysis

---

## Critical Actions (Week 1-2)

### CI/CD Pipeline

- [ ] Create `.github/workflows/ci.yml` with:
  - [ ] Build job (cargo check, cargo build)
  - [ ] Test job (cargo test --all)
  - [ ] Clippy job (cargo clippy -- -D warnings)
  - [ ] Format check (cargo fmt --check)

- [ ] Create `.github/workflows/security.yml` with:
  - [ ] cargo-audit for dependency vulnerabilities
  - [ ] cargo-deny for license compliance

### Logging Fixes

- [ ] Respect `sage_config.json` logging settings in CLI main.rs:232
- [ ] Add JSON output format option
- [ ] Add sensitive data redaction for API keys/tokens
- [ ] Enable file output with rotation

---

## High Priority Actions (Week 3-4)

### Code Consolidation

- [ ] Merge dual `ModelParameters` definitions:
  - Source 1: `sage-core/src/config/model.rs`
  - Source 2: `sage-core/src/llm/providers.rs`
  - Target: Single `types/model.rs`

### Trajectory Refactor

- [ ] Implement stubbed methods in `trajectory/recorder.rs`:
  - [ ] `load_trajectory()` (line 347)
  - [ ] `list_trajectories()` (line 367)
  - [ ] `delete_trajectory()` (line 381)
  - [ ] `get_statistics()` (line 401)

### Containerization

- [ ] Create `Dockerfile` (multi-stage build)
- [ ] Create `.dockerignore`
- [ ] Create `docker-compose.yml` for development

### Provider Streaming

- [ ] Implement streaming for:
  - [ ] Google (line 2027)
  - [ ] Azure (line 2036)
  - [ ] OpenRouter (line 2045)
  - [ ] Doubao (line 2054)
  - [ ] Ollama (line 2063)
  - [ ] GLM (line 2072)

---

## Medium Priority Actions (Week 5-8)

### Large File Refactoring

| File | Lines | Action |
|------|-------|--------|
| `llm/client.rs` | 2,075 | Extract provider-specific modules |
| `session/types.rs` | 1,240 | Decompose by domain |
| `tools/permission.rs` | 1,204 | Extract permission strategies |
| `input/mod.rs` | 1,097 | Separate channel types |

### Documentation

- [ ] Create `docs/user-guide/getting-started.md`
- [ ] Create `docs/user-guide/configuration.md`
- [ ] Create `docs/user-guide/cli-reference.md`
- [ ] Create `docs/development/setup.md`
- [ ] Create `docs/development/contributing.md`
- [ ] Add rustdoc examples to public APIs

### Tool Implementations

- [ ] Complete `network/web_fetch.rs` (line 48)
- [ ] Complete `network/web_search.rs` (line 79)
- [ ] Complete diagnostics tools (4 stubs)

---

## Low Priority Actions (Week 9-12)

### Observability

- [ ] Add OpenTelemetry integration
- [ ] Add Prometheus metrics exporter
- [ ] Add health check endpoints
- [ ] Define SLOs/SLIs

### Code Quality

- [ ] Reduce `unwrap()` calls (1,414 total)
- [ ] Add more unit tests for edge cases
- [ ] Add property-based testing

### Documentation Completion

- [ ] Document remaining 29+ tools
- [ ] Add API reference with examples
- [ ] Create troubleshooting guide

---

## Quick Reference

### Key Files to Monitor

```
sage-core/src/llm/client.rs          # 2,075 lines - needs refactoring
sage-core/src/trajectory/recorder.rs  # Contains stubbed methods
sage-cli/src/main.rs                  # Logging initialization
sage-core/src/tools/executor.rs       # Missing features (deps, resources, sandbox)
```

### Commands

```bash
# Run all tests
cargo test --all

# Check with clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Security audit
cargo audit

# Generate docs
cargo doc --open
```

### CI/CD Status Goals

| Check | Current | Target |
|-------|---------|--------|
| Build | Manual | Automated on every PR |
| Tests | Manual | Automated on every PR |
| Security | None | Automated weekly |
| Coverage | Unknown | >80% |
| Releases | Manual | Automated on tag |

---

*Generated 2025-12-22*

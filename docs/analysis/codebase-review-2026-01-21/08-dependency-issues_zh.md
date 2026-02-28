# Dependency Issues Analysis

**Generated:** 2026-01-21
**Codebase:** Sage Agent

## Summary

This report analyzes dependencies for security, versioning, and optimization opportunities.

---

## 1. Dependency Overview

### Workspace Configuration
- **Resolver:** Version 2
- **Crates:** 4 (sage-core, sage-cli, sage-sdk, sage-tools)
- **Workspace Version:** 0.5.1

### Dependency Categories
| Category | Count | Key Dependencies |
|----------|-------|------------------|
| Async Runtime | 3 | tokio, tokio-util, futures |
| HTTP | 1 | reqwest |
| Serialization | 4 | serde, serde_json, serde_yaml, toml |
| CLI | 4 | clap, console, indicatif, dialoguer |
| Error Handling | 2 | anyhow, thiserror |
| Logging | 2 | tracing, tracing-subscriber |

---

## 2. Security Concerns

### Issue 1: Outdated Dependency Versions

**Concern:** Some dependencies may have known vulnerabilities.

**Recommendation:**
```bash
# Run security audit
cargo audit

# Update dependencies
cargo update
```

### Issue 2: Path Dependencies

**Location:** Root Cargo.toml
```toml
rnk = { path = "/Users/apple/Desktop/code/AI/tool/tink" }
```

**Concern:**
- Absolute path dependency won't work for other developers
- Not reproducible builds

**Recommendation:**
1. Publish `rnk` to crates.io
2. Or use relative path: `path = "../rnk"`
3. Or use git dependency: `git = "https://github.com/..."`

---

## 3. Version Pinning

### Current State
Most dependencies use semantic versioning without pinning:
```toml
tokio = { version = "1.0" }
serde = { version = "1.0" }
```

### Recommendations
1. **Production builds**: Consider exact versions in Cargo.lock
2. **CI/CD**: Always commit Cargo.lock
3. **Security updates**: Regular `cargo update` with testing

---

## 4. Duplicate Dependencies

### Potential Issues
- Multiple crates may pull different versions of transitive dependencies
- Check with: `cargo tree --duplicates`

### Recommendations
1. Regular deduplication review
2. Use workspace dependencies consistently
3. Upgrade transitive dependencies when needed

---

## 5. Feature Flag Analysis

### Tokio Features
```toml
tokio = { version = "1.0", features = ["full"] }
```

**Concern:** `full` includes all features, some may be unnecessary.

**Recommendation:**
```toml
# Minimal feature set
tokio = { version = "1.0", features = [
    "rt-multi-thread",
    "fs",
    "io-util",
    "net",
    "sync",
    "time",
    "macros",
] }
```

### Reqwest Features
```toml
reqwest = { version = "0.12", features = ["json", "stream"] }
```

**Status:** Good - only necessary features enabled.

---

## 6. Unused Dependencies

### Detection
```bash
# Find unused dependencies
cargo +nightly udeps
```

### Potential Candidates
- Review `console` vs `colored` (potential overlap)
- Check if all serialization crates are needed

---

## 7. Heavy Dependencies

### Large Crates
| Crate | Impact | Notes |
|-------|--------|-------|
| reqwest | High | Full HTTP stack |
| tokio | High | Full async runtime |
| serde | Medium | Serialization framework |

### Build Time Impact
- Consider `cargo build --timings` to identify slow builds
- Use `sccache` or `mold` linker for faster builds

---

## 8. Licensing

### Current Licenses
| Crate | License |
|-------|---------|
| tokio | MIT |
| serde | MIT/Apache-2.0 |
| reqwest | MIT/Apache-2.0 |
| clap | MIT/Apache-2.0 |

**Status:** Compatible - all dependencies use permissive licenses.

---

## 9. Maintenance Status

### Check Dependency Health
- Last update date
- GitHub stars/activity
- Issue response time

### Recommended Review
| Crate | Status | Action |
|-------|--------|--------|
| parking_lot | Active | Good |
| rnk | Local | Document or publish |
| notify | Active | Good |

---

## 10. Recommendations

### Immediate Actions
1. **Fix path dependency**: Replace absolute path for `rnk`
2. **Run security audit**: `cargo audit`
3. **Update lock file**: `cargo update`

### Short-term
4. **Optimize tokio features**: Reduce from "full"
5. **Check for unused deps**: `cargo +nightly udeps`
6. **Review duplicates**: `cargo tree --duplicates`

### Long-term
7. **Establish update policy**: Monthly dependency review
8. **Add CI checks**: cargo audit in CI pipeline
9. **Document dependencies**: Why each is needed

---

## Dependency Checklist

- [ ] No absolute path dependencies
- [ ] Security audit passing
- [ ] Cargo.lock committed
- [ ] No unused dependencies
- [ ] Minimal feature flags
- [ ] Compatible licenses
- [ ] Dependencies actively maintained

---

## Commands for Review

```bash
# Security audit
cargo audit

# Find duplicates
cargo tree --duplicates

# Find unused (requires nightly)
cargo +nightly udeps

# Build timing
cargo build --timings

# Update all
cargo update
```

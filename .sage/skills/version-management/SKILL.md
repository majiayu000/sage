---
name: version-management
description: Sage 版本管理规范，包含语义化版本、CHANGELOG、发布流程
when_to_use: 当需要更新版本号、发布新版本、查看版本历史时使用
allowed_tools:
  - Read
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 95
---

# Sage 版本管理规范

## 版本号格式

采用语义化版本 (SemVer): `MAJOR.MINOR.PATCH`

- **MAJOR**: 不兼容的 API 变更
- **MINOR**: 向后兼容的功能新增
- **PATCH**: 向后兼容的 bug 修复

当前阶段使用 `0.1.x` 系列，每次更新递增 PATCH。

## 版本更新流程

### 1. 更新版本号

所有 crate 使用统一版本，需要更新以下文件：

```bash
# 根 Cargo.toml (workspace)
version = "0.1.X"

# workspace.package 中的版本
[workspace.package]
version = "0.1.X"
```

### 2. 更新命令

```bash
# 使用 cargo-edit (推荐)
cargo set-version 0.1.X

# 或手动编辑 Cargo.toml
```

### 3. 版本号文件位置

| 文件 | 说明 |
|------|------|
| `Cargo.toml` | 根 workspace 版本 |
| `crates/sage-core/Cargo.toml` | 继承 workspace 版本 |
| `crates/sage-cli/Cargo.toml` | 继承 workspace 版本 |
| `crates/sage-sdk/Cargo.toml` | 继承 workspace 版本 |
| `crates/sage-tools/Cargo.toml` | 继承 workspace 版本 |

### 4. CHANGELOG 更新

每次版本更新必须更新 `CHANGELOG.md`:

```markdown
## [0.1.X] - YYYY-MM-DD

### Added
- 新功能描述

### Changed
- 变更描述

### Fixed
- 修复描述

### Removed
- 移除的功能
```

## 版本更新检查清单

发布前确认：

- [ ] 所有 Cargo.toml 版本号已更新
- [ ] CHANGELOG.md 已更新
- [ ] 所有测试通过 (`cargo test`)
- [ ] Clippy 无警告 (`cargo clippy`)
- [ ] 代码已格式化 (`cargo fmt`)
- [ ] Git tag 已创建

## Git Tag 规范

```bash
# 创建版本 tag
git tag -a v0.1.X -m "Release v0.1.X: 简短描述"

# 推送 tag
git push origin v0.1.X
```

## 自动版本更新脚本

在 `scripts/` 目录创建 `bump-version.sh`:

```bash
#!/bin/bash
NEW_VERSION=$1

if [ -z "$NEW_VERSION" ]; then
    echo "Usage: ./scripts/bump-version.sh 0.1.X"
    exit 1
fi

# 更新 Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# 运行 cargo check 验证
cargo check

echo "Version bumped to $NEW_VERSION"
echo "Don't forget to:"
echo "  1. Update CHANGELOG.md"
echo "  2. Commit changes"
echo "  3. Create git tag: git tag -a v$NEW_VERSION -m 'Release v$NEW_VERSION'"
```

## 版本号递增规则

### 0.1.x 阶段（当前）

每次有意义的更新都递增 PATCH:
- 0.1.0 → 0.1.1 → 0.1.2 → ... → 0.1.66

### 0.2.x 阶段（Tink UI 重构后）

重构完成后升级到 0.2.0，表示重大架构变更。

### 1.0.0 阶段（正式发布）

当 API 稳定后发布 1.0.0。

## 发布到 Crates.io

```bash
# 登录 (首次)
cargo login

# 发布 (按依赖顺序)
cargo publish -p sage-core
cargo publish -p sage-tools
cargo publish -p sage-sdk
cargo publish -p sage-cli
```

## 版本查询

```bash
# 查看当前版本
cargo pkgid

# 查看所有 crate 版本
cargo metadata --format-version 1 | jq '.packages[] | select(.name | startswith("sage")) | {name, version}'
```

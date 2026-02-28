# Sage Agent 综合审计 TODO 清单

> 基于 project-health-auditor 审计结果和 `.sage/skills/` 分析生成
> 日期: 2026-01-11

---

## 项目健康评分: 7.7/10

| 分类 | 评分 | 状态 |
|------|------|------|
| 代码质量 | 7/10 | 需改进 |
| 依赖健康 | 8/10 | 良好 |
| 安全性 | 8/10 | 良好 |
| 测试覆盖 | 6/10 | 需改进 |
| 文档完整 | 9/10 | 优秀 |
| 架构设计 | 8/10 | 良好 |

---

## P0 - 必须立即修复

### 1. 大文件拆分 (违反 200 行限制)

根据 `sage-architecture` 和 `sage-rust-conventions` Skills，以下文件必须拆分：

| 文件 | 当前行数 | 目标 | 状态 |
|------|---------|------|------|
| `sage-cli/src/commands/unified.rs` | **1300** | <200 | [ ] |
| `sage-core/src/config/provider.rs` | 882 | <200 | [ ] |
| `sage-core/src/skills/types.rs` | 783 | <200 | [ ] |
| `sage-core/src/config/credential/resolver.rs` | 711 | <200 | [ ] |
| `sage-core/src/config/onboarding/manager.rs` | 702 | <200 | [ ] |
| `sage-core/src/config/model_params.rs` | 658 | <200 | [ ] |
| `sage-core/src/config/credential/unified_loader.rs` | 660 | <200 | [ ] |
| `sage-core/src/llm/sse_decoder.rs` | 609 | <200 | [ ] |
| `sage-core/src/plugins/registry.rs` | 609 | <200 | [ ] |

**拆分建议**：
- `unified.rs` → `unified/{mod,handlers,parsers,utils}.rs`
- `provider.rs` → `provider/{mod,types,builder,validation}.rs`
- `types.rs` → `types/{mod,skill,trigger,access}.rs`

### 2. 修复 `/commit` Skill 合规性

**问题**：`.sage/skills/commit/SKILL.md` 使用了 `Co-Authored-By`，违反 `CLAUDE.md` 规范

**修复方案**：
```diff
- Co-Authored-By: Sage Agent <noreply@sage.dev>
+ Signed-off-by: majiayu000 <1835304752@qq.com>
```

- [ ] 修改 `.sage/skills/commit/SKILL.md`
- [ ] 移除 Co-Authored-By
- [ ] 添加 Signed-off-by 示例

### 3. Clippy 警告修复

```bash
cargo clippy --fix --lib -p sage-core
```

当前警告：
- [ ] `.as_ref().map(|v| v.as_slice())` 可简化
- [ ] 多余的引用创建
- [ ] 手动剥离前缀可用 `strip_prefix`
- [ ] 应使用 `sort_by_key`

---

## P1 - 重要改进

### 4. 依赖统一

| 依赖 | 当前版本 | 统一版本 | 状态 |
|------|---------|---------|------|
| `base64` | 0.21.7, 0.22.1 | 0.22.1 | [ ] |

### 5. Skill 系统集成到执行循环

根据 `sage-agent-execution` Skill 指南：

- [ ] 在 `UnifiedExecutor::execute_step()` 中注入匹配的 Skill prompt
- [ ] 实现 `SkillContext::from_execution_context()`
- [ ] 添加 Skill 激活日志

### 6. 上下文文件发现扩展

根据 `sage-skill-gap-analysis.md`，当前仅支持 `SAGE.md`，需扩展：

- [ ] 支持 `CLAUDE.md` (Claude Code 兼容)
- [ ] 支持 `.cursorrules` (Cursor 兼容)
- [ ] 支持 `.github/copilot-instructions.md` (Copilot 兼容)
- [ ] 支持 `README.md` 中的特定标记

### 7. Token 预算管理

根据 Claude Code 最佳实践 (15000 字符限制)：

- [ ] 在 `skills/registry.rs` 添加 `SKILL_CHAR_BUDGET = 15000`
- [ ] 实现 `generate_skills_xml_with_budget()` 函数
- [ ] 按优先级截断 Skills

---

## P2 - 优化增强

### 8. 测试覆盖率

- [ ] 安装 `cargo-tarpaulin` 统计覆盖率
- [ ] 为核心模块添加单元测试
- [ ] 目标：行覆盖率 > 60%

当前测试文件数：24 个

### 9. TODO 注释清理

发现 **45+** 个 TODO 注释，主要在 `test_generator` 模块：

- [ ] 审查所有 TODO 注释
- [ ] 实现或删除过时的 TODO
- [ ] 将有价值的 TODO 转为 Issue

### 10. Skill 性能优化

根据 `sage-skill-development` 指南：

- [ ] 添加 `trigger_index: HashMap<SkillTrigger, Vec<String>>`
- [ ] 添加 `extension_index: HashMap<String, Vec<String>>`
- [ ] 添加 `match_cache: LruCache<SkillContext, Vec<String>>`

### 11. 新增 Skills

根据 gap 分析建议：

| Skill | 优先级 | 触发条件 | 状态 |
|-------|--------|----------|------|
| `test-generation` | P0 | TaskType::Testing | [ ] |
| `documentation` | P0 | "doc", "readme" | [ ] |
| `performance-analysis` | P1 | "perf", "slow" | [ ] |
| `pr-creation` | P1 | "pr", "pull request" | [ ] |
| `refactoring` | P2 | TaskType::Refactoring | [ ] |

---

## 架构优化 (基于 sage-architecture Skill)

### 12. 模块合并

| 当前 | 合并到 | 原因 | 状态 |
|-----|-------|------|------|
| `settings/` | `config/` | 功能重叠 | [ ] |
| `session/` | `context/` | 会话是上下文的一部分 | [ ] |
| `sandbox/` | `tools/sandbox/` | 沙箱服务于工具执行 | [ ] |
| `modes/` | `agent/modes/` | 模式是 agent 的状态 | [ ] |
| `cost/` | `telemetry/cost/` | 成本是遥测的一部分 | [ ] |

### 13. TOOL.md 描述文件

根据 `sage-tool-development` 指南，学习 Crush 模式：

- [ ] 为每个工具创建 `TOOL.md` 描述文件
- [ ] 实现描述文件自动注入 system prompt
- [ ] 格式参考 `sage-tool-development` Skill

---

## Sage 独有优势强化

### 14. 恢复模式系统 (sage-recovery-patterns)

- [ ] 完善 Circuit Breaker 文档和示例
- [ ] 添加 Rate Limiter 配置指南
- [ ] 集成 Supervisor 到 Agent 执行循环

### 15. 学习系统 (sage-learning-system)

- [ ] 实现 `LearningEngine::record_event()` 端到端流程
- [ ] 添加模式检测器注册机制
- [ ] 集成偏好到 system prompt

### 16. 检查点系统 (sage-checkpoint-system)

- [ ] 实现 `/checkpoint` 斜杠命令
- [ ] 添加工具执行前自动检查点
- [ ] 实现增量存储和压缩

---

## 已有 Skills 清单

### 项目级 Skills (.sage/skills/) - 19 个

| Skill | 优先级 | 描述 |
|-------|--------|------|
| sage-architecture | 100 | 项目整体架构设计指南 |
| sage-core-module-guide | 95 | sage-core 模块职责详解 |
| sage-agent-execution | 95 | Agent 执行引擎开发指南 |
| sage-llm-integration | 94 | LLM 客户端集成指南 |
| sage-config-system | 93 | 配置系统开发指南 |
| sage-tool-development | 90 | 工具开发规范 |
| sage-skill-development | 90 | Skill 开发规范 |
| sage-mcp-protocol | 90 | MCP 协议开发指南 |
| sage-rust-conventions | 85 | Rust 代码规范 |
| sage-prompt-engineering | 85 | Prompt 工程规范 |
| sage-recovery-patterns | 80 | 恢复模式指南 (独有) |
| sage-learning-system | 75 | 学习系统指南 (独有) |
| sage-checkpoint-system | 75 | 检查点系统指南 (独有) |
| sage-sandbox-security | 70 | 沙箱安全指南 |
| sage-session-management | 70 | 会话管理指南 |
| sage-context-management | 70 | 上下文管理指南 |
| sage-workspace-detection | 70 | 工作区检测指南 |
| commit | 10 | Git 提交工作流 |
| review-code | 8 | 代码审查专家 |

---

## 实施路线图

### 第一阶段：基础修复 (本周)

1. [ ] 拆分 `unified.rs` (P0)
2. [ ] 修复 `/commit` Skill (P0)
3. [ ] 修复 Clippy 警告 (P0)
4. [ ] 统一 base64 依赖 (P1)

### 第二阶段：能力增强 (下周)

1. [ ] Skill 系统集成到执行循环
2. [ ] 扩展上下文文件发现
3. [ ] 新增 test-generation Skill
4. [ ] 新增 documentation Skill

### 第三阶段：架构优化 (后续)

1. [ ] 模块合并 (settings → config 等)
2. [ ] 为工具创建 TOOL.md
3. [ ] 实现 Token 预算管理
4. [ ] 添加 Skill 性能缓存

### 第四阶段：差异化强化 (持续)

1. [ ] 深化恢复模式系统文档
2. [ ] 完善学习系统集成
3. [ ] 强化检查点系统
4. [ ] 建立 Skill 社区生态

---

## 竞争定位

### Sage 独有优势 (Claude Code / Cursor / Crush 均无)

| 功能 | 模块 | 状态 |
|------|------|------|
| 熔断器/限流器 | recovery/ | 已实现 |
| 模式学习/偏好 | learning/ | 已实现 |
| 状态快照/回滚 | checkpoints/ | 已实现 |
| 安全沙箱 | sandbox/ | 已实现 |
| 多存储后端 | storage/ | 已实现 |

### 差异化口号

> **Sage Agent**: 可靠的、会学习的、可回滚的 AI 编程伙伴

---

*此 TODO 清单将根据项目发展持续更新*

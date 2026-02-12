---
name: auto-optimize
description: 跨语言代码库健康治理与自动优化系统。基于学术研究和实战经验，系统性检测 LLM 生成代码的结构性缺陷。完整流程：SCAN → DIAGNOSE → FIX → HARDEN。适用于任何语言的项目。
when_to_use: 当需要系统性分析和优化代码库、防止 AI vibe coding 退化、执行代码健康检查、或批量修复架构问题时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
  - Task
user_invocable: true
argument_hint: "[target project path or 'self' for current project] [scan|full]"
priority: 95
---

# Anti-Vibe-Coding: 跨语言代码库健康治理系统

## 核心理念

LLM 生成代码有五个结构性缺陷（基于 IEEE-ISTAS 2025、arxiv 2503.06327、arxiv 2512.18020）：

1. **局部最优陷阱** → 跨模块类型/函数重复
2. **上下文断裂分叉** → 并行实现分叉
3. **保守性退化** → deprecated shim 堆积、死代码增长
4. **迭代安全衰减** → 5 轮迭代后关键漏洞增加 37.6%
5. **架构无感** → 破坏安全边界、信任模型（15/20 AI 补全含架构缺陷）

**防御原则**: Policy-as-code > prompt engineering。

---

## 规则体系（数据驱动）

规则定义与执行逻辑分离，存放在 `rules/` 目录：

```
rules/
├── universal.md   # 通用规则 U-01 ~ U-23（所有语言必加载）
├── rust.md        # Rust 特定 RS-01 ~ RS-09
├── python.md      # Python 特定 PY-01 ~ PY-08
├── typescript.md  # TypeScript 特定 TS-01 ~ TS-08
└── go.md          # Go 特定 GO-01 ~ GO-06
```

每个规则文件包含：
- 规则 ID + 描述
- 来源（学术论文/OWASP/实战案例）
- 检测模式（grep pattern 或 lint rule）

**扩展方式**：添加新语言只需创建 `rules/<lang>.md`，无需修改 SKILL.md。
**项目自定义**：项目可在自己的 CLAUDE.md 中追加 NEVER 规则，扫描时自动合并。

---

## 执行流程

```
SCAN → DIAGNOSE → FIX → HARDEN → (repeat)
```

---

## Phase 1: SCAN（扫描）

### 1.0 项目探测

自动检测项目语言和工具链，决定加载哪些规则文件：

```
检测逻辑：
1. 读取项目根目录文件列表
2. 匹配项目标识文件（每个 rules/<lang>.md 顶部定义了标识文件）
   - Cargo.toml → 加载 rules/rust.md
   - package.json → 加载 rules/typescript.md
   - pyproject.toml / setup.py → 加载 rules/python.md
   - go.mod → 加载 rules/go.md
3. 始终加载 rules/universal.md
4. 读取项目 CLAUDE.md，提取额外 NEVER 规则合并
5. 从语言规则文件中读取 lint/test/format 命令
```

多语言项目（如 monorepo）可同时加载多个规则集。

### 1.1 自动化健康检查

对每条已加载的规则，使用其定义的检测模式执行扫描：

```
对于每条规则 R in (universal + 语言特定):
  如果 R 有 grep 检测模式:
    运行 grep，统计违规数和位置
  如果 R 依赖 lint 工具:
    运行语言规则文件中定义的 lint 命令
  如果 R 需要 AI 分析:
    标记为"需深度扫描"，留给 Phase 1.2
```

同时收集基础指标：
- 文件行数分布（wc -l）
- 导出符号数量（pub/export 统计）
- TODO/FIXME 数量
- lint 警告数
- 测试通过率

### 1.2 AI 深度扫描（定期或按需）

启动 3 个并行子 agent，处理 grep 无法覆盖的规则：

**Agent 1: 结构分析**
- 模块依赖图（循环依赖检测）
- 幽灵模块（目录存在但未声明/导入）
- pub 接口 vs 实际调用（死代码候选）

**Agent 2: 语义重复 + 安全审计**
- U-02 功能重复（语义级，非文本级）
- U-06~U-10 安全边界深度检查
- 依赖健康度（CVE、幻觉依赖）

**Agent 3: 模式违规 + 迭代衰减**
- 所有标记为"需 AI 分析"的规则
- 错误处理审计（U-11~U-13）
- 最近 N 次 commit 的 lint 警告趋势（迭代衰减信号）

每个 agent 输出结构化报告：
```markdown
| ID | 规则 | 严重度 | 位置 | 描述 |
|----|------|--------|------|------|
```

---

## Phase 2: DIAGNOSE（诊断）

### 优先级分类

```
P0 (立即修复): 安全漏洞(U-06~U-10)、编译错误、数据丢失风险
P1 (本轮修复): 类型重复(U-01)、代码分叉(U-02)、死代码>300行(U-05)
P2 (下轮修复): 文件超200行(U-16)、命名不一致、缺失测试
P3 (记录不修): 风格偏好、注释缺失、文档过时
```

### 生成诊断报告

合并所有扫描结果，输出到 `docs/todo/<date>/health-scan-report.md`：

```markdown
# 代码库健康诊断报告 [日期]

## 项目信息
- 语言: [auto-detected]
- 加载的规则集: universal + [语言]
- 项目自定义规则: [从 CLAUDE.md 提取的数量]

## 健康指标
| 指标 | 当前值 | 阈值 | 状态 |
|------|--------|------|------|

## 问题清单（标注规则 ID）
## 建议修复顺序
```

---

## Phase 3: FIX（修复）

### 修复协议

```
1. 创建修复分支（问题 > 3 个时）
2. 按 P0 → P1 → P2 顺序，每个问题：
   a. 写/更新测试（红灯）
   b. 修复代码（绿灯）
   c. 运行 lint（从语言规则文件读取命令）
   d. 运行测试（从语言规则文件读取命令）
   e. 单独 commit
3. 全量回归测试
4. 合并到 main
```

### 修复策略

| 问题类型 | 修复策略 | 禁止策略 |
|---------|---------|---------|
| 类型重复 | 移到共享模块，原位置 re-export | 创建 type alias |
| 代码分叉 | 提取共享模块，删除旧实现 | 保留两套 + deprecated |
| 文件膨胀 | 按职责拆分子模块 | 只加注释不拆分 |
| 死代码 | 直接删除 | 注释掉保留 |
| 安全漏洞 | 修复 + 添加测试 | 加 TODO 推迟 |

**核心原则：不做向后兼容。Breaking change 可接受，更新版本号。**

### 并行修复约束

- 每个 agent 负责不同文件集合，禁止交叉修改
- 禁止子 agent 执行 git checkout/reset
- 主 agent 统一 commit
- 有依赖的修复必须串行

---

## Phase 4: HARDEN（加固）

### 规则回写

每次修复一个 AI 引入的 bug：

```
1. 识别 bug 模式
2. 归类到已有规则 → 记录新案例
   或创建新规则 → 分配 ID，写入对应 rules/<lang>.md
3. 如果可自动检测 → 在规则文件中添加检测模式
4. 同步到项目 CLAUDE.md（LLM 可读）和 CI（自动化）
```

### MEMORY.md 更新

```markdown
## [日期] 健康扫描结果
- 语言: [detected]
- 规则集: universal + [lang] (共 N 条)
- 发现 N 个问题，已修复 M 个
- 新增规则：[ID + 描述]
- 测试基线：X/Y 通过
```

---

## 健康指标

| 指标 | 阈值 | 适用 |
|------|------|------|
| 类型重复率 | 0% | 全部 |
| 文件膨胀率 | ≤5% | 全部 |
| NEVER 违规数 | 0 | 全部 |
| 安全问题数 | 0 | 全部 |
| Lint 警告数 | 0 | 全部 |
| 测试通过率 | 100% | 全部 |
| 死代码行数 | ≤100 | 全部 |
| 迭代衰减率 | ≤0% | 全部 |

---

## 使用方式

```
/auto-optimize self          # 扫描当前项目（自动检测语言）
/auto-optimize /path/to/proj # 扫描指定项目
/auto-optimize self full     # 完整 SCAN → DIAGNOSE → FIX → HARDEN
/auto-optimize self scan     # 只扫描，生成报告
```

执行时根据 $ARGUMENTS 判断目标项目和执行模式。默认对当前项目执行快速扫描。

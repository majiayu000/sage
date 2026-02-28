# SWE-bench 评估分析与长期记忆系统设计

## 1. 执行摘要

本文档基于 Sage Agent 在 SWE-bench 基准测试上的历史运行数据，分析现有问题并设计长期记忆系统以提升 Agent 性能。

### 关键发现

| 指标 | 数值 | 说明 |
|------|------|------|
| 总测试实例 | 300 | SWE-bench_Lite 数据集 |
| 提交实例 | 10 | 完成评估流程 |
| 成功解决 | 3 | 通过测试验证 |
| 解决率 | 30% | 3/10 |
| 空 Patch 率 | 10% | 1/10 |
| 错误率 | 40% | 4/10 |

---

## 2. 问题分析

### 2.1 空 Patch 问题

**受影响实例**: `django__django-10924`, `django__django-11039`, `django__django-11179`, `django__django-11283`

**根本原因分析**:

1. **代码定位失败**: Agent 无法准确找到需要修改的源文件
2. **问题理解不足**: 未能正确理解问题描述并转化为代码修改
3. **工具使用错误**: 创建了临时文件而非修改源码
4. **超时退出**: 在生成 patch 前超时

**具体案例**:
- `django__django-10924` (FilePathField callable path): Agent 尝试修改但 patch 为空
- `django__django-11039`: 完全未能产生任何代码修改

### 2.2 测试失败问题

**受影响实例**: `astropy__astropy-14182`, `astropy__astropy-14365`

**失败模式**:

1. **语义错误**: 修改了错误的代码逻辑
2. **不完整修复**: 只修复了部分问题
3. **回归引入**: 修复引入了新的错误

### 2.3 执行错误

**受影响实例**: `astropy__astropy-6938`, `astropy__astropy-7746`, `django__django-11001`, `django__django-11019`

**错误类型**:
- Patch 应用失败
- 测试环境配置问题
- 依赖兼容性问题

---

## 3. 成功案例分析

### 3.1 成功解决的实例

| 实例 ID | 问题类型 | 修复策略 |
|---------|----------|----------|
| `astropy__astropy-12907` | 数组操作错误 | 单行修复，`= 1` → `= right` |
| `astropy__astropy-14995` | 条件判断遗漏 | 添加空 mask 处理分支 |
| `django__django-10914` | 默认值问题 | 修改配置默认值 |

### 3.2 成功模式总结

1. **问题范围小**: 修改局限于 1-2 个文件
2. **修改量少**: 通常少于 10 行代码
3. **模式明确**: 有清晰的 bug 描述和预期行为
4. **熟悉领域**: Django/Astropy 项目结构相对清晰

---

## 4. 现有系统分析

### 4.1 当前 Memory 系统

```
位置: crates/sage-core/src/memory/

组件:
├── types.rs      - Memory, MemoryType, MemoryCategory, MemoryQuery
├── storage.rs    - FileMemoryStorage 持久化
├── manager.rs    - MemoryManager 核心管理
└── mod.rs        - 模块导出

MemoryType:
- Fact           (事实信息)
- Preference     (用户偏好)
- CodeContext    (代码上下文)
- ConversationSummary (对话摘要)
- TaskHistory    (任务历史)
- Lesson         (经验教训)
- Custom         (自定义)
```

### 4.2 当前 Learning 系统

```
位置: crates/sage-core/src/learning/

组件:
├── types.rs      - Pattern, PatternType, Confidence, LearningConfig
├── patterns.rs   - PatternDetector, 风格分析
├── engine.rs     - LearningEngine 核心引擎
└── mod.rs        - 模块导出

PatternType:
- Correction           (纠错)
- ToolPreference       (工具偏好)
- CodingStyle          (编码风格)
- ErrorHandling        (错误处理)
- CommunicationStyle   (沟通风格)
- WorkflowPreference   (工作流偏好)
- ProjectSpecific      (项目特定)
```

### 4.3 现有系统的局限性

1. **无跨项目学习**: 在 Django 学到的模式无法应用到 Astropy
2. **无失败分析**: 不会自动从失败案例中学习
3. **无代码模式库**: 没有积累常见 bug 修复模式
4. **检索效率低**: 基于文本匹配，缺乏语义理解
5. **未与执行集成**: Memory/Learning 与 Agent 执行松耦合

---

## 5. 长期记忆系统设计方案

### 方案对比概览

| 方案 | 复杂度 | 效果预期 | 开发周期 | 维护成本 |
|------|--------|----------|----------|----------|
| A. 增强型本地存储 | 低 | 中 | 短 | 低 |
| B. 向量数据库 RAG | 中 | 高 | 中 | 中 |
| C. 图谱知识库 | 高 | 高 | 长 | 高 |
| D. 混合架构 | 中 | 最高 | 中 | 中 |

---

### 方案 A: 增强型本地存储

**核心思路**: 在现有 Memory/Learning 系统基础上增强

```
架构:
┌─────────────────────────────────────────┐
│              Sage Agent                  │
├─────────────────────────────────────────┤
│     Memory Manager (Enhanced)            │
│  ┌─────────┬─────────┬─────────┐        │
│  │ Session │ Project │ Global  │        │
│  │ Memory  │ Memory  │ Memory  │        │
│  └────┬────┴────┬────┴────┬────┘        │
│       │         │         │              │
│  ┌────▼─────────▼─────────▼────┐        │
│  │     Unified Storage Layer    │        │
│  │   (JSON/SQLite with Index)   │        │
│  └──────────────────────────────┘        │
└─────────────────────────────────────────┘
```

**新增组件**:

1. **FixPatternLibrary** - 常见修复模式库
```rust
struct FixPattern {
    id: PatternId,
    bug_type: BugType,           // regex_issue, off_by_one, null_check, etc.
    symptoms: Vec<String>,       // 问题特征
    fix_template: String,        // 修复模板
    project_types: Vec<String>,  // 适用项目类型
    success_count: u32,
    confidence: f32,
}
```

2. **FailureAnalyzer** - 失败案例分析器
```rust
struct FailureRecord {
    instance_id: String,
    failure_type: FailureType,
    root_cause: String,
    attempted_fix: String,
    lesson: String,
    timestamp: DateTime<Utc>,
}
```

3. **ProjectKnowledge** - 项目知识库
```rust
struct ProjectKnowledge {
    project_name: String,        // django, astropy, etc.
    directory_structure: HashMap<String, String>,
    key_patterns: Vec<CodePattern>,
    common_issues: Vec<IssuePattern>,
    test_commands: Vec<String>,
}
```

**优点**:
- 实现简单，可快速迭代
- 与现有系统兼容
- 无额外依赖

**缺点**:
- 检索能力有限
- 规模扩展性一般

---

### 方案 B: 向量数据库 RAG

**核心思路**: 使用向量嵌入实现语义检索

```
架构:
┌─────────────────────────────────────────────────┐
│                   Sage Agent                     │
├─────────────────────────────────────────────────┤
│              RAG Memory Layer                    │
│  ┌──────────────────────────────────────────┐   │
│  │           Query Processing                │   │
│  │  ┌─────────┐  ┌──────────┐  ┌─────────┐  │   │
│  │  │ Encode  │→ │  Search  │→ │ Retrieve│  │   │
│  │  └─────────┘  └──────────┘  └─────────┘  │   │
│  └──────────────────────────────────────────┘   │
│                      │                           │
│  ┌───────────────────▼──────────────────────┐   │
│  │          Vector Database                  │   │
│  │  ┌────────┐  ┌────────┐  ┌────────────┐  │   │
│  │  │Code    │  │Fix     │  │Project     │  │   │
│  │  │Patterns│  │History │  │Knowledge   │  │   │
│  │  └────────┘  └────────┘  └────────────┘  │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

**技术选型**:

| 组件 | 选项 1 | 选项 2 | 推荐 |
|------|--------|--------|------|
| 向量数据库 | Qdrant | LanceDB | LanceDB (嵌入式) |
| 嵌入模型 | OpenAI Ada | BGE-M3 | BGE-M3 (本地) |
| 检索策略 | Dense | Hybrid | Hybrid |

**核心数据结构**:

```rust
struct VectorMemory {
    id: String,
    content: String,
    embedding: Vec<f32>,
    memory_type: MemoryType,
    metadata: HashMap<String, Value>,
    created_at: DateTime<Utc>,
}

struct RetrievalQuery {
    query_text: String,
    query_embedding: Option<Vec<f32>>,
    filters: Vec<Filter>,
    top_k: usize,
    min_score: f32,
}
```

**优点**:
- 语义检索能力强
- 支持跨项目知识迁移
- 可扩展性好

**缺点**:
- 需要额外依赖
- 嵌入计算有延迟
- 需要维护向量索引

---

### 方案 C: 图谱知识库

**核心思路**: 构建代码/修复知识图谱

```
架构:
┌───────────────────────────────────────────────────┐
│                    Sage Agent                      │
├───────────────────────────────────────────────────┤
│               Knowledge Graph Layer                │
│  ┌─────────────────────────────────────────────┐  │
│  │              Graph Database                  │  │
│  │                                              │  │
│  │   [Project]──has_module──>[Module]          │  │
│  │       │                      │               │  │
│  │   has_pattern            contains            │  │
│  │       │                      │               │  │
│  │       ▼                      ▼               │  │
│  │   [Pattern]◄──fixes──[BugType]              │  │
│  │       │                      │               │  │
│  │   similar_to            causes               │  │
│  │       │                      │               │  │
│  │       ▼                      ▼               │  │
│  │   [Pattern]──────────>[Symptom]              │  │
│  └─────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

**图谱实体**:

```rust
enum GraphEntity {
    Project { name: String, repo: String },
    Module { path: String, description: String },
    BugType { category: String, description: String },
    FixPattern { template: String, success_rate: f32 },
    Symptom { description: String, indicators: Vec<String> },
}

enum GraphRelation {
    HasModule,
    Contains,
    Fixes,
    Causes,
    SimilarTo,
    LearnedFrom,
}
```

**优点**:
- 关系推理能力强
- 可解释性好
- 支持复杂查询

**缺点**:
- 实现复杂度高
- 需要图数据库
- 知识抽取困难

---

### 方案 D: 混合架构 (推荐)

**核心思路**: 结合方案 A 和 B 的优点

```
架构:
┌──────────────────────────────────────────────────────────┐
│                      Sage Agent                           │
├──────────────────────────────────────────────────────────┤
│                 Hybrid Memory System                      │
│                                                           │
│  ┌─────────────────┐      ┌─────────────────────────┐    │
│  │  Fast Cache     │      │   Semantic Index        │    │
│  │  (Recent/Hot)   │      │   (LanceDB Embedded)    │    │
│  │                 │      │                         │    │
│  │  - Last 100     │      │  - Code Patterns        │    │
│  │  - High Conf    │◄────►│  - Fix Templates        │    │
│  │  - Pinned       │      │  - Project Knowledge    │    │
│  └────────┬────────┘      └───────────┬─────────────┘    │
│           │                           │                   │
│           └───────────┬───────────────┘                   │
│                       │                                   │
│           ┌───────────▼───────────┐                       │
│           │   Unified Query API    │                       │
│           │                        │                       │
│           │  query(context) →      │                       │
│           │    [relevant_memories] │                       │
│           └────────────────────────┘                       │
│                       │                                   │
│           ┌───────────▼───────────┐                       │
│           │  Persistent Storage   │                       │
│           │  (~/.sage/memory/)    │                       │
│           │                       │                       │
│           │  - memories.json      │                       │
│           │  - patterns.json      │                       │
│           │  - projects.json      │                       │
│           │  - embeddings.lance   │                       │
│           └───────────────────────┘                       │
└──────────────────────────────────────────────────────────┘
```

**核心模块设计**:

```rust
// 统一记忆管理器
pub struct HybridMemoryManager {
    // 快速缓存层
    fast_cache: FastCache,
    // 语义索引层
    semantic_index: SemanticIndex,
    // 持久化层
    storage: MemoryStorage,
    // 学习引擎
    learning: LearningEngine,
}

impl HybridMemoryManager {
    /// 智能检索相关记忆
    pub async fn retrieve(&self, context: &RetrievalContext) -> Vec<RelevantMemory> {
        let mut results = Vec::new();

        // 1. 快速检索高优先级记忆
        results.extend(self.fast_cache.get_hot_memories(context));

        // 2. 语义检索相关模式
        if let Some(patterns) = self.semantic_index.search(context).await {
            results.extend(patterns);
        }

        // 3. 排序和去重
        self.rank_and_dedupe(results)
    }

    /// 从执行结果学习
    pub async fn learn_from_execution(&mut self, outcome: &ExecutionOutcome) {
        match outcome.status {
            Status::Success => {
                // 记录成功模式
                self.record_success_pattern(outcome).await;
            }
            Status::Failure(reason) => {
                // 分析失败原因并记录教训
                self.analyze_and_record_failure(outcome, reason).await;
            }
        }
    }
}
```

**记忆类型扩展**:

```rust
// 新增的记忆类型
pub enum EnhancedMemoryType {
    // 现有类型...

    /// 代码修复模式
    FixPattern {
        bug_category: String,
        symptoms: Vec<String>,
        fix_template: String,
        applicability: f32,
    },

    /// 项目结构知识
    ProjectStructure {
        project_type: String,
        key_directories: HashMap<String, String>,
        entry_points: Vec<String>,
    },

    /// 失败案例记录
    FailureLesson {
        failure_type: String,
        root_cause: String,
        prevention: String,
    },

    /// 工具使用模式
    ToolUsagePattern {
        tool_name: String,
        effective_scenarios: Vec<String>,
        anti_patterns: Vec<String>,
    },
}
```

**检索策略**:

```rust
pub struct RetrievalContext {
    /// 当前任务描述
    pub task_description: String,
    /// 当前项目类型
    pub project_type: Option<String>,
    /// 涉及的文件
    pub relevant_files: Vec<String>,
    /// 错误信息
    pub error_context: Option<String>,
    /// 检索限制
    pub max_results: usize,
}

pub struct RelevantMemory {
    pub memory: Memory,
    pub relevance_score: f32,
    pub retrieval_method: RetrievalMethod,
}

pub enum RetrievalMethod {
    FastCache,
    SemanticSearch,
    PatternMatch,
    Hybrid,
}
```

---

## 6. 实施建议

### 阶段 1: 基础增强 (1-2 周)

1. **增强现有 Memory 类型**
   - 添加 `FixPattern`, `FailureLesson` 类型
   - 扩展 `MemoryQuery` 支持更多过滤条件

2. **实现失败分析器**
   - 解析 SWE-bench 执行结果
   - 自动生成失败记录

3. **创建项目知识模板**
   - Django 项目结构模板
   - Astropy 项目结构模板

### 阶段 2: 语义增强 (2-3 周)

1. **集成 LanceDB**
   - 添加 Cargo 依赖
   - 实现嵌入式向量存储

2. **实现嵌入生成**
   - 集成本地嵌入模型 (BGE-M3 / all-MiniLM)
   - 或使用 OpenAI 嵌入 API

3. **构建 Hybrid 检索**
   - 实现统一查询接口
   - 融合关键词和语义检索

### 阶段 3: 学习闭环 (2-3 周)

1. **执行后学习**
   - Hook 到 Agent 执行流程
   - 自动记录成功/失败

2. **主动应用**
   - 在 Agent 启动时加载相关记忆
   - 动态调整 system prompt

3. **效果评估**
   - A/B 测试框架
   - 记忆有效性追踪

---

## 7. 预期效果

### 性能提升预测

| 指标 | 当前 | 阶段 1 后 | 阶段 2 后 | 阶段 3 后 |
|------|------|-----------|-----------|-----------|
| 解决率 | 30% | 40% | 50% | 60%+ |
| 空 Patch 率 | 10% | 5% | 3% | 1% |
| 平均解决时间 | - | -10% | -20% | -30% |

### ROI 分析

- **开发投入**: ~4-8 周
- **预期收益**: 解决率提升 30%+
- **维护成本**: 低到中等

---

## 8. 附录

### A. 数据文件位置

```
swebench_eval/
├── sage-agent.eval_10.json      # 评估汇总结果
├── predictions_10.json          # 10 个实例的 patch
├── swebench_runs/               # 详细运行记录
│   ├── astropy__astropy-12907/
│   ├── django__django-10914/
│   └── ...
└── run_agent.py                 # 评估脚本
```

### B. 关键代码位置

```
crates/sage-core/src/
├── memory/                      # 现有记忆系统
│   ├── types.rs
│   ├── storage.rs
│   └── manager.rs
└── learning/                    # 现有学习系统
    ├── types.rs
    ├── patterns.rs
    └── engine.rs
```

### C. 参考资源

- SWE-bench 官方: https://github.com/princeton-nlp/SWE-bench
- LanceDB: https://lancedb.github.io/lancedb/
- BGE-M3: https://huggingface.co/BAAI/bge-m3

---

*文档生成时间: 2025-12-25*
*分析基于: sage-agent.eval_10.json, predictions_10.json, 及 trajectories 目录下的历史记录*

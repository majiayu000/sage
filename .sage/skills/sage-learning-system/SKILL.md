---
name: sage-learning-system
description: Sage 独有的学习系统设计，包含模式识别、用户偏好学习、纠正反馈机制
when_to_use: 当需要实现或扩展学习功能、理解用户偏好、或设计自适应系统时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
user_invocable: true
priority: 75
---

# Sage 学习系统指南

## 概述

Sage 的 `learning/` 模块是**独有的竞争优势**，提供：

- **模式检测**: 识别用户行为模式
- **偏好学习**: 学习用户代码风格和偏好
- **纠正记录**: 记录和学习用户纠正
- **自适应调整**: 根据反馈调整行为

## 模块结构

```
learning/
├── mod.rs              # 公开接口
├── engine/             # 学习引擎
│   ├── engine.rs       # 核心引擎
│   └── config.rs       # 配置
├── patterns/           # 模式检测
│   ├── detector.rs     # 模式检测器
│   └── types.rs        # 模式类型
└── types/              # 类型定义
    ├── correction.rs   # 纠正记录
    ├── pattern.rs      # 模式定义
    └── preference.rs   # 偏好定义
```

## 模式类型

### 代码风格模式

```rust
pub enum StylePattern {
    /// 命名风格
    Naming {
        convention: NamingConvention,  // snake_case, camelCase, etc.
        context: NamingContext,        // variable, function, type, etc.
    },

    /// 缩进风格
    Indentation {
        style: IndentStyle,    // Spaces, Tabs
        size: usize,           // 2, 4, etc.
    },

    /// 注释风格
    Comments {
        style: CommentStyle,   // Line, Block, Doc
        density: CommentDensity,
    },

    /// 导入组织
    Imports {
        grouping: ImportGrouping,
        ordering: ImportOrdering,
    },
}
```

### 行为模式

```rust
pub enum BehaviorPattern {
    /// 工具使用偏好
    ToolPreference {
        tool: String,
        frequency: f32,
        context: String,
    },

    /// 响应详细程度偏好
    VerbosityPreference {
        level: VerbosityLevel,  // Minimal, Normal, Detailed
        context: String,
    },

    /// 确认偏好
    ConfirmationPreference {
        requires_confirmation: bool,
        operation_type: String,
    },
}
```

## 学习引擎

### 初始化

```rust
use sage_core::learning::{LearningEngine, LearningConfig};

let config = LearningConfig {
    // 模式检测阈值
    pattern_threshold: 3,  // 出现 3 次才认定为模式

    // 置信度衰减
    confidence_decay: 0.1,  // 每次未使用衰减 10%

    // 存储配置
    storage_path: PathBuf::from("~/.config/sage/learning"),

    // 最大模式数
    max_patterns: 1000,
};

let engine = LearningEngine::new(config).await?;
```

### 记录事件

```rust
use sage_core::learning::{LearningEvent, LearningEventType};

// 记录用户纠正
engine.record_event(LearningEvent {
    event_type: LearningEventType::Correction,
    data: CorrectionData {
        original: "function getName()".to_string(),
        corrected: "fn get_name()".to_string(),
        context: "rust_code".to_string(),
    },
    timestamp: Utc::now(),
}).await?;

// 记录工具使用
engine.record_event(LearningEvent {
    event_type: LearningEventType::ToolUsage,
    data: ToolUsageData {
        tool: "Grep".to_string(),
        success: true,
        duration: Duration::from_millis(150),
    },
    timestamp: Utc::now(),
}).await?;
```

### 检测模式

```rust
// 获取检测到的模式
let patterns = engine.detect_patterns().await?;

for pattern in patterns {
    println!("Pattern: {:?}", pattern.pattern_type);
    println!("Confidence: {:.2}", pattern.confidence);
    println!("Occurrences: {}", pattern.occurrences);
}
```

### 应用学习

```rust
// 根据学习结果调整行为
if let Some(naming_pattern) = engine.get_pattern(PatternType::Naming).await? {
    if naming_pattern.confidence > 0.8 {
        // 高置信度，应用学习到的命名风格
        apply_naming_convention(naming_pattern.convention);
    }
}

// 获取用户偏好
let verbosity = engine.get_preference::<VerbosityPreference>().await?;
set_response_verbosity(verbosity.level);
```

## 纠正记录系统

### 记录纠正

```rust
use sage_core::learning::CorrectionRecord;

let correction = CorrectionRecord {
    id: CorrectionId::new(),

    // 原始内容
    original: OriginalContent {
        text: "const API_URL = 'http://api.example.com'".to_string(),
        file_path: Some("src/config.rs".into()),
        line_range: Some(10..12),
    },

    // 纠正后内容
    corrected: CorrectedContent {
        text: "const API_URL: &str = \"http://api.example.com\";".to_string(),
    },

    // 元数据
    reason: Some("Rust 字符串使用双引号，类型显式声明".to_string()),
    timestamp: Utc::now(),
    context: CorrectionContext::RustCode,
};

engine.record_correction(correction).await?;
```

### 分析纠正趋势

```rust
// 获取纠正统计
let stats = engine.get_correction_stats().await?;

println!("Total corrections: {}", stats.total);
println!("By category:");
for (category, count) in &stats.by_category {
    println!("  {}: {}", category, count);
}

// 获取常见纠正模式
let common_corrections = engine.get_common_corrections(10).await?;
for correction in common_corrections {
    println!("{} -> {}", correction.pattern, correction.replacement);
}
```

## 模式检测器

### 实现自定义检测器

```rust
use sage_core::learning::{PatternDetector, DetectionResult};

pub struct NamingPatternDetector {
    samples: Vec<NamingSample>,
}

impl PatternDetector for NamingPatternDetector {
    type Pattern = NamingPattern;

    fn feed(&mut self, event: &LearningEvent) {
        if let Some(naming) = extract_naming(event) {
            self.samples.push(naming);
        }
    }

    fn detect(&self) -> Vec<DetectionResult<Self::Pattern>> {
        let mut results = Vec::new();

        // 分析样本找出模式
        let conventions = analyze_conventions(&self.samples);

        for (convention, count) in conventions {
            if count >= 3 {
                results.push(DetectionResult {
                    pattern: NamingPattern { convention },
                    confidence: calculate_confidence(count, self.samples.len()),
                    occurrences: count,
                });
            }
        }

        results
    }
}
```

### 注册检测器

```rust
let mut engine = LearningEngine::new(config).await?;

// 注册自定义检测器
engine.register_detector(Box::new(NamingPatternDetector::new()));
engine.register_detector(Box::new(IndentationPatternDetector::new()));
engine.register_detector(Box::new(ToolPreferenceDetector::new()));
```

## 偏好系统

### 定义偏好

```rust
use sage_core::learning::{Preference, PreferenceIndicator};

#[derive(Preference)]
pub struct CodeStylePreference {
    /// 命名约定
    #[preference(indicator = PreferenceIndicator::FromCorrections)]
    pub naming: NamingConvention,

    /// 缩进风格
    #[preference(indicator = PreferenceIndicator::FromFiles)]
    pub indentation: IndentStyle,

    /// 最大行长度
    #[preference(default = 100)]
    pub max_line_length: usize,
}
```

### 推断偏好

```rust
// 从用户行为推断偏好
let inferred = engine.infer_preferences::<CodeStylePreference>().await?;

println!("Inferred naming: {:?}", inferred.naming);
println!("Inferred indentation: {:?}", inferred.indentation);
```

### 用户显式设置

```rust
// 用户显式设置偏好
engine.set_preference("code_style.naming", NamingConvention::SnakeCase).await?;
engine.set_preference("code_style.max_line_length", 120).await?;
```

## 与其他模块集成

### 与 Agent 集成

```rust
impl AgentExecutor {
    pub async fn execute_with_learning(&self, task: &Task) -> Result<()> {
        // 1. 获取相关学习数据
        let patterns = self.learning.get_relevant_patterns(task).await?;

        // 2. 调整行为
        let adjusted_options = self.apply_patterns(patterns);

        // 3. 执行
        let result = self.execute_inner(task, adjusted_options).await?;

        // 4. 记录结果用于学习
        self.learning.record_execution(task, &result).await?;

        Ok(())
    }
}
```

### 与 Prompts 集成

```rust
// 在 system prompt 中注入学习到的偏好
fn build_system_prompt(learning: &LearningEngine) -> String {
    let mut prompt = base_prompt();

    if let Some(style) = learning.get_preference::<CodeStylePreference>() {
        prompt.push_str(&format!(
            "\n<user_preferences>\n\
             Naming convention: {:?}\n\
             Indentation: {:?}\n\
             </user_preferences>\n",
            style.naming, style.indentation
        ));
    }

    prompt
}
```

### 与 Memory 集成

```rust
// 长期记忆中存储学习数据
impl LearningEngine {
    pub async fn persist_to_memory(&self, memory: &MemoryManager) -> Result<()> {
        let patterns = self.get_all_patterns().await?;

        for pattern in patterns {
            memory.store(Memory {
                id: MemoryId::new(),
                category: MemoryCategory::Learning,
                content: serde_json::to_string(&pattern)?,
                metadata: MemoryMetadata {
                    source: MemorySource::Learning,
                    confidence: pattern.confidence,
                    ..Default::default()
                },
            }).await?;
        }

        Ok(())
    }
}
```

## 最佳实践

### 1. 渐进式学习

```rust
// 不要立即应用低置信度模式
if pattern.confidence < CONFIDENCE_THRESHOLD {
    // 继续收集数据
    continue;
}

// 高置信度模式可以自动应用
if pattern.confidence > AUTO_APPLY_THRESHOLD {
    apply_pattern(pattern);
}
```

### 2. 衰减机制

```rust
// 长期未使用的模式置信度衰减
impl LearningEngine {
    pub async fn decay_unused_patterns(&mut self) {
        let now = Utc::now();

        for pattern in &mut self.patterns {
            let days_unused = (now - pattern.last_used).num_days();
            if days_unused > 7 {
                pattern.confidence *= 0.9;  // 每周衰减 10%
            }
        }

        // 移除低置信度模式
        self.patterns.retain(|p| p.confidence > MIN_CONFIDENCE);
    }
}
```

### 3. 用户反馈优先

```rust
// 显式纠正优先于推断
if let Some(explicit) = user_preferences.get(key) {
    return explicit.clone();
}

if let Some(inferred) = learning.infer(key) {
    return inferred;
}

default_value()
```

### 4. 隐私保护

```rust
// 不记录敏感信息
fn sanitize_for_learning(content: &str) -> String {
    let mut sanitized = content.to_string();

    // 移除 API keys
    sanitized = API_KEY_REGEX.replace_all(&sanitized, "[REDACTED]").to_string();

    // 移除密码
    sanitized = PASSWORD_REGEX.replace_all(&sanitized, "[REDACTED]").to_string();

    sanitized
}
```

## 配置建议

```rust
LearningConfig {
    // 模式检测阈值（出现次数）
    pattern_threshold: 3,

    // 自动应用置信度阈值
    auto_apply_threshold: 0.8,

    // 最小保留置信度
    min_confidence: 0.1,

    // 置信度衰减率（每周）
    weekly_decay_rate: 0.1,

    // 最大模式数
    max_patterns: 1000,

    // 最大纠正记录数
    max_corrections: 5000,

    // 存储路径
    storage_path: PathBuf::from("~/.config/sage/learning"),
}
```

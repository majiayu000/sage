# 代码库冗余/重复设计检查

本报告基于当前代码结构与注释状态，列出明显的冗余设计与重复实现点，并给出对应定位路径与初步影响判断。

## 1) Git 工具重复实现

- 简化版 Git 工具：`crates/sage-tools/src/tools/vcs/git_simple.rs`
- 完整版 Git 工具：`crates/sage-tools/src/tools/vcs/git/mod.rs`

问题：两套实现都覆盖 status/add/commit/push/pull/branch/diff 等常用操作，功能交叠且维护成本翻倍。长期会导致行为不一致和测试分裂。

## 2) 任务管理两套体系并行

- Claude 风格 Todo 系列：
  - `crates/sage-tools/src/tools/task_mgmt/todo_write.rs`
  - `crates/sage-tools/src/tools/task_mgmt/todo_read.rs`
  - `crates/sage-tools/src/tools/task_mgmt/task_done.rs`
- 结构化 TaskList 体系：
  - `crates/sage-tools/src/tools/task_mgmt/task_management/task_list.rs`

问题：两套模型各自有全局存储，状态定义/操作 API 不一致，属于明显重复设计。

## 3) 编辑工具重叠（含禁用保留模块）

- 单文件 Edit 工具：`crates/sage-tools/src/tools/file_ops/edit.rs`
- 批量 MultiEdit 工具（标注 DISABLED）：`crates/sage-tools/src/tools/file_ops/multi_edit/mod.rs`

问题：两者都做字符串替换编辑，MultiEdit 只是批量化，但当前标注禁用且长期保留会形成冗余路径。

## 4) 代码检索工具重叠（含禁用保留模块）

- Grep 搜索：`crates/sage-tools/src/tools/file_ops/grep/mod.rs`
- CodebaseRetrieval（标注 DISABLED）：`crates/sage-tools/src/tools/file_ops/codebase_retrieval/mod.rs`

问题：二者都做代码检索，只是策略不同；当前禁用模块长期保留会增加复杂度。

## 5) CLI 新旧 UI 并行

- legacy UI 标记与开关：
  - `crates/sage-cli/src/args.rs`
  - `crates/sage-cli/src/app.rs`
  - `crates/sage-cli/src/signal_handler.rs`
- 新 UI 实现：`crates/sage-cli/src/ui/rnk_app/`
- 旧 UI 组件：`crates/sage-cli/src/ui/` 下其他模块

问题：新旧 UI 双轨维护，属于长期重复设计风险点。

---

## 结论

当前仓库存在多处“新旧/增强/简化”并行实现，其中部分已标注禁用但仍保留。建议在后续规划中明确：
- 哪一套是主路径（默认注册/默认入口）
- 哪一套是待迁移/待删除路径
- 删除或合并的时间表与兼容策略


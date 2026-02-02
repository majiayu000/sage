# NotebookEdit Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | NotebookEdit |
| Claude Code 版本 | 2.0.14 |
| 类别 | 文件操作 |
| Sage 实现 | `sage-tools/src/tools/file_ops/notebook_edit/` |

## 功能描述

编辑 Jupyter notebook (.ipynb) 文件中的单元格，支持替换、插入和删除操作。

## 完整 Prompt

```markdown
Completely replaces the contents of a specific cell in a Jupyter notebook (.ipynb file) with new source. Jupyter notebooks are interactive documents that combine code, text, and visualizations, commonly used for data analysis and scientific computing. The notebook_path parameter must be an absolute path, not a relative path. The cell_number is 0-indexed. Use edit_mode=insert to add a new cell at the index specified by cell_number. Use edit_mode=delete to delete the cell at the index specified by cell_number.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| notebook_path | string | ✅ | notebook 文件的绝对路径 |
| new_source | string | ✅* | 新的单元格内容 (*delete 模式不需要) |
| cell_id | string | ❌ | 目标单元格 ID |
| cell_type | string | ❌ | 单元格类型: "code" 或 "markdown" |
| edit_mode | string | ❌ | 编辑模式: "replace" (默认), "insert", "delete" |

## 设计原理

### 1. 绝对路径要求
**为什么**:
- 与其他文件工具保持一致
- 避免路径解析错误
- 明确的文件定位

### 2. 0-indexed 单元格编号
**为什么**:
- 与编程惯例一致
- 与 Jupyter 内部索引一致
- 便于程序化操作

### 3. 三种编辑模式
**为什么**:
- `replace`: 修改现有单元格
- `insert`: 添加新单元格
- `delete`: 删除单元格
- 覆盖常见编辑场景

### 4. 支持 cell_id
**为什么**:
- 单元格可能被重新排序
- ID 比索引更稳定
- 精确定位目标单元格

## 使用场景

### ✅ 应该使用
- 修改 notebook 中的代码单元格
- 添加新的分析步骤
- 更新 markdown 文档
- 删除不需要的单元格

### ❌ 不应该使用
- 创建新 notebook (使用 Write)
- 读取 notebook 内容 (使用 Read)
- 批量修改多个单元格

## 示例

### 替换单元格
```json
{
  "notebook_path": "/path/to/analysis.ipynb",
  "cell_id": "abc123",
  "new_source": "import pandas as pd\ndf = pd.read_csv('data.csv')",
  "edit_mode": "replace"
}
```

### 插入新单元格
```json
{
  "notebook_path": "/path/to/analysis.ipynb",
  "cell_id": "abc123",
  "new_source": "# Data Visualization\nimport matplotlib.pyplot as plt",
  "cell_type": "code",
  "edit_mode": "insert"
}
```

### 删除单元格
```json
{
  "notebook_path": "/path/to/analysis.ipynb",
  "cell_id": "abc123",
  "new_source": "",
  "edit_mode": "delete"
}
```

## Jupyter Notebook 结构

```json
{
  "cells": [
    {
      "cell_type": "markdown",
      "id": "abc123",
      "source": ["# Title"]
    },
    {
      "cell_type": "code",
      "id": "def456",
      "source": ["print('hello')"],
      "outputs": []
    }
  ],
  "metadata": {...},
  "nbformat": 4,
  "nbformat_minor": 5
}
```

## Sage 实现差异

Sage 的 NotebookEditTool 与 Claude Code 基本一致，额外支持：
- 单元格输出清理
- 元数据更新
- 批量单元格操作

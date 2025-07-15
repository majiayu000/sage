# Documentation Consistency Management

This document outlines the strategy and tools for maintaining consistency between English and Chinese documentation versions.

## 🎯 Goals

- Ensure content parity between English and Chinese versions
- Maintain synchronized updates across languages
- Provide clear workflow for documentation maintainers
- Automate consistency checks where possible
- Track documentation status and identify gaps

## 📁 File Structure

### Current Structure
```
docs/
├── README.md                    # English main documentation index
├── README_zh.md                 # Chinese main documentation index
├── user-guide/
│   ├── getting-started.md       # English user guide
│   └── getting-started_zh.md    # Chinese user guide (to be created)
├── development/
│   ├── contributing.md          # English development docs
│   └── contributing_zh.md       # Chinese development docs (to be created)
└── ...
```

### Recommended Structure
```
docs/
├── en/                          # English documentation
│   ├── README.md
│   ├── user-guide/
│   ├── development/
│   ├── architecture/
│   ├── api/
│   └── planning/
├── zh/                          # Chinese documentation
│   ├── README.md
│   ├── user-guide/
│   ├── development/
│   ├── architecture/
│   ├── api/
│   └── planning/
└── shared/                      # Language-agnostic content
    ├── images/
    ├── diagrams/
    └── code-examples/
```

## 🔄 Synchronization Strategy / 同步策略

### 1. Master Language Approach / 主语言方法

**English as Master / 英文为主**
- English documentation is the source of truth
- Chinese documentation follows English updates
- All new features documented in English first
- Chinese translation follows within 48-72 hours

**中文跟随英文**
- 英文文档作为权威来源
- 中文文档跟随英文更新
- 所有新功能首先用英文记录
- 中文翻译在48-72小时内跟进

### 2. Version Control Integration / 版本控制集成

**Git Hooks / Git钩子**
```bash
# Pre-commit hook to check documentation consistency
#!/bin/bash
# Check if English docs changed without corresponding Chinese updates
python scripts/check_doc_consistency.py
```

**Branch Strategy / 分支策略**
- `main` branch: Both languages must be in sync
- `docs/en-update` branch: English documentation updates
- `docs/zh-update` branch: Chinese translation updates
- `docs/sync` branch: Synchronization work

## 🛠️ Tools and Automation / 工具和自动化

### 1. Consistency Checker Script / 一致性检查脚本

Create `scripts/check_doc_consistency.py`:
```python
#!/usr/bin/env python3
"""
Documentation consistency checker for English and Chinese docs.
中英文文档一致性检查器
"""

import os
import re
from pathlib import Path
from datetime import datetime

class DocConsistencyChecker:
    def __init__(self, docs_root="docs"):
        self.docs_root = Path(docs_root)
        self.en_dir = self.docs_root / "en"
        self.zh_dir = self.docs_root / "zh"
    
    def check_file_parity(self):
        """Check if all English files have Chinese counterparts"""
        en_files = self.get_doc_files(self.en_dir)
        zh_files = self.get_doc_files(self.zh_dir)
        
        missing_zh = en_files - zh_files
        missing_en = zh_files - en_files
        
        return {
            'missing_chinese': missing_zh,
            'missing_english': missing_en,
            'status': len(missing_zh) == 0 and len(missing_en) == 0
        }
    
    def check_content_freshness(self):
        """Check if Chinese docs are up-to-date with English"""
        # Implementation for checking last modified dates
        pass
    
    def generate_report(self):
        """Generate consistency report"""
        pass
```

### 2. Translation Workflow / 翻译工作流程

**GitHub Actions Workflow**
```yaml
name: Documentation Consistency Check
on:
  pull_request:
    paths:
      - 'docs/**'
  push:
    branches: [main]

jobs:
  check-consistency:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Check Documentation Consistency
        run: python scripts/check_doc_consistency.py
      - name: Create Issue if Inconsistent
        if: failure()
        uses: actions/github-script@v6
        with:
          script: |
            github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: 'Documentation Consistency Issue',
              body: 'Chinese documentation needs to be updated to match English changes.'
            })
```

### 3. Translation Management / 翻译管理

**Translation Status Tracking**
```markdown
# Translation Status / 翻译状态

| File | English Updated | Chinese Updated | Status | Translator |
|------|----------------|-----------------|---------|------------|
| README.md | 2025-01-15 | 2025-01-15 | ✅ Synced | @translator1 |
| user-guide/getting-started.md | 2025-01-14 | 2025-01-13 | ⚠️ Outdated | @translator2 |
| development/contributing.md | 2025-01-15 | - | ❌ Missing | - |
```

## 📋 Workflow Process / 工作流程

### For Documentation Updates / 文档更新流程

1. **English Update / 英文更新**
   ```bash
   # Create branch for English documentation update
   git checkout -b docs/en-update-feature-x
   
   # Update English documentation
   vim docs/en/user-guide/new-feature.md
   
   # Commit with special tag for translation tracking
   git commit -m "docs(en): add new feature documentation [needs-translation]"
   ```

2. **Translation Request / 翻译请求**
   - Automated issue creation for translation needed
   - Assign to Chinese documentation maintainer
   - Include context and priority level

3. **Chinese Translation / 中文翻译**
   ```bash
   # Create branch for Chinese translation
   git checkout -b docs/zh-translate-feature-x
   
   # Translate the documentation
   vim docs/zh/user-guide/new-feature.md
   
   # Commit with reference to original
   git commit -m "docs(zh): translate new feature documentation (refs: commit-hash)"
   ```

4. **Synchronization Check / 同步检查**
   ```bash
   # Run consistency checker
   python scripts/check_doc_consistency.py
   
   # Update translation status
   python scripts/update_translation_status.py
   ```

### For Major Updates / 重大更新流程

1. **Planning Phase / 规划阶段**
   - Create documentation update plan in both languages
   - Assign translators and reviewers
   - Set timeline for completion

2. **Parallel Development / 并行开发**
   - English and Chinese teams work simultaneously
   - Regular sync meetings to ensure consistency
   - Shared terminology glossary

3. **Review and QA / 审查和质量保证**
   - Cross-language review process
   - Technical accuracy verification
   - Cultural adaptation review

## 🔍 Quality Assurance / 质量保证

### 1. Terminology Consistency / 术语一致性

**Glossary Management / 术语表管理**
```markdown
# Technical Terminology / 技术术语

| English | Chinese | Context | Notes |
|---------|---------|---------|-------|
| Agent | 智能体 | AI agent system | Preferred over "代理" |
| Tool | 工具 | Agent tools | - |
| Execution | 执行 | Task execution | - |
| Trajectory | 轨迹 | Execution path | - |
| LLM | 大语言模型 | Large Language Model | Keep acronym + translation |
```

### 2. Review Process / 审查流程

**Multi-stage Review / 多阶段审查**
1. **Technical Review / 技术审查**: Verify technical accuracy
2. **Language Review / 语言审查**: Check grammar and style
3. **Cultural Review / 文化审查**: Ensure cultural appropriateness
4. **Consistency Review / 一致性审查**: Cross-language consistency check

### 3. Automated Checks / 自动化检查

**Content Validation / 内容验证**
- Link validation across languages
- Code example consistency
- Image and diagram synchronization
- Table of contents alignment

## 📊 Metrics and Monitoring / 指标和监控

### Key Metrics / 关键指标
- Translation lag time (English update → Chinese update)
- Documentation coverage percentage
- Consistency score between languages
- User feedback on documentation quality

### Monitoring Dashboard / 监控仪表板
- Real-time translation status
- Outdated documentation alerts
- Translator workload distribution
- Quality metrics tracking

## 🚀 Implementation Plan / 实施计划

### Phase 1: Infrastructure Setup / 基础设施搭建
- [ ] Reorganize documentation structure
- [ ] Create consistency checking scripts
- [ ] Set up GitHub Actions workflows
- [ ] Establish translation status tracking

### Phase 2: Process Implementation / 流程实施
- [ ] Train team on new workflow
- [ ] Create translator guidelines
- [ ] Implement review processes
- [ ] Set up monitoring systems

### Phase 3: Optimization / 优化
- [ ] Gather feedback and iterate
- [ ] Automate more consistency checks
- [ ] Improve translation tools
- [ ] Scale to more languages if needed

---

**Maintainers / 维护者**: Documentation Team
**Last Updated / 最后更新**: 2025-01-15
**Review Schedule / 审查计划**: Monthly

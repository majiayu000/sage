# SWE-bench 成功案例模式分析报告

## 概述

本报告分析了3个成功通过测试的SWE-bench任务的trajectory文件，找出成功的关键模式和最佳实践。

---

## 一、任务基本信息

### 1.1 任务对比

| 任务 | 步数 | 执行时间 | 成功率 | 修改前步数 | 效率(步/分) |
|------|------|----------|--------|------------|------------|
| django-10914 | 16 | 66秒 (1.1分钟) | 100% | 11 | 14.47 |
| astropy-14995 | 38 | 214秒 (3.6分钟) | 100% | 13 | 10.64 |
| astropy-12907 | 70 | 417秒 (7.0分钟) | 100% | 22 | 10.07 |

### 1.2 Token消耗

| 任务 | 输入Token | 输出Token | 总Token |
|------|-----------|-----------|---------|
| django-10914 | 470,757 | 1,861 | 472,618 |
| astropy-14995 | 1,473,663 | 6,299 | 1,479,962 |
| astropy-12907 | 2,724,606 | 15,312 | 2,739,918 |

**关键发现**：
- 最高效的任务（django-10914）只用了11步就定位到问题，总共16步完成
- Token消耗与任务复杂度成正比，但输出Token占比很小（<1%）

---

## 二、工具使用模式

### 2.1 工具使用统计

| 任务 | Bash | Read | Grep | Glob | Edit | Write | TodoWrite |
|------|------|------|------|------|------|-------|-----------|
| django-10914 | 3 | 4 | 4 | 4 | 1 | 0 | 4 |
| astropy-14995 | 17 | 10 | 5 | 2 | 1 | 2 | 0 |
| astropy-12907 | 49 | 9 | 2 | 6 | 1 | 3 | 0 |

### 2.2 关键发现

**成功模式1：精准的搜索策略**
- 所有任务都在前5步内开始搜索关键代码
- 使用Grep搜索关键类名、函数名、配置项
- 使用Glob定位文件位置
- 使用Read读取候选文件内容

**成功模式2：工具执行零错误**
- 所有3个任务的工具调用成功率都是100%
- 没有任何工具执行错误（is_error = false）
- 没有agent级别的错误（error字段为null）

**成功模式3：测试驱动的验证**
- astropy-14995: 8次测试验证（步骤15, 16, 17, 19, 20, 22, 23, 32）
- astropy-12907: 20次测试验证（贯穿整个过程）
- django-10914: 无显式测试命令（可能是简单配置修改）

---

## 三、执行流程分析

### 3.1 Django-10914 (最高效，16步完成)

**任务**: 设置FILE_UPLOAD_PERMISSION默认值为0o644

**执行流程**:
1. **步骤1**: TodoWrite - 创建任务列表
2. **步骤2-3**: Grep + Glob - 搜索FILE_UPLOAD_PERMISSIONS和FileSystemStorage
3. **步骤4**: Bash - 使用find定位相关文件
4. **步骤5**: Read - 读取storage.py, uploadhandler.py, uploadedfile.py
5. **步骤6**: Grep - 再次确认FILE_UPLOAD_PERMISSIONS使用位置
6. **步骤7**: Bash - 查找所有使用FILE_UPLOAD_PERMISSIONS的文件
7. **步骤8**: Glob - 定位global_settings.py
8. **步骤9**: Read - 读取global_settings.py
9. **步骤10**: TodoWrite - 更新任务状态
10. **步骤11**: Edit - 修改global_settings.py (关键修改)
    ```python
    # 修改前
    FILE_UPLOAD_PERMISSIONS = None
    # 修改后
    FILE_UPLOAD_PERMISSIONS = 0o644
    ```
11. **步骤12-13**: Bash, TodoWrite - 验证和更新状态
12. **步骤14**: TodoWrite - 标记完成
13. **步骤15**: task_done - 完成任务
14. **步骤16**: 状态变为Completed

**关键成功因素**:
- 使用TodoWrite明确任务进度（步骤1, 10, 12, 14）
- 多轮搜索快速收敛到正确文件（步骤2-9）
- 单次精准修改，没有回退
- 11步定位 + 1步修改 + 4步验证/收尾

### 3.2 Astropy-14995 (中等复杂度，38步完成)

**任务**: 修复NDDataRef mask传播失败问题

**执行流程**:
1. **步骤1-12**: 搜索阶段
   - Grep搜索NDDataRef类和multiply方法
   - Glob定位nddata相关文件
   - Read读取关键文件（nddata.py, ndarithmetic.py等）

2. **步骤13**: 核心修改
   - Edit: astropy/nddata/mixins/ndarithmetic.py

3. **步骤14**: 创建测试文件
   - Write: test_mask_fix.py

4. **步骤15-27**: 测试验证循环
   - 多次运行测试验证修复效果
   - 8次测试执行，确保修复正确

5. **步骤28**: 创建额外验证脚本
   - Write: verify_fix.py

6. **步骤29-38**: 最终验证和收尾

**关键成功因素**:
- 前12步精准定位问题代码
- 立即创建测试文件验证修复
- 多轮测试确保修复可靠
- 使用Write创建测试脚本而不是临时验证

### 3.3 Astropy-12907 (高复杂度，70步完成)

**任务**: 修复separability_matrix的复合模型处理问题

**执行流程**:
1. **步骤1-21**: 深度搜索和理解阶段
   - Glob定位separable.py和compound模型相关文件
   - 多次Read理解代码结构
   - 10次bash命令运行测试用例理解问题

2. **步骤22**: 核心修改
   - Edit: astropy/modeling/separable.py

3. **步骤23-45**: 密集测试验证
   - 20次测试执行
   - 创建多个测试文件（test_fix.py, test_logic.py）
   - 创建FIX_EXPLANATION.md文档

4. **步骤46-70**: 持续验证和完善

**关键成功因素**:
- 充分的前期理解（21步探索）
- 使用bash多次运行复现脚本理解问题
- 密集的测试验证（20次测试）
- 创建文档记录修复逻辑

---

## 四、成功模式总结

### 4.1 核心成功要素

1. **系统化的搜索策略** (前30-50%的步骤)
   - Grep搜索关键词（类名、函数名、配置项）
   - Glob定位文件路径
   - Read读取候选文件
   - Bash运行复现脚本理解问题

2. **精准的代码修改** (1次Edit成功)
   - 所有3个任务都只用了1次Edit修改核心代码
   - 没有修改回退或重试
   - 修改前充分理解问题

3. **充分的测试验证**
   - 简单任务：无需测试（配置修改）
   - 中等任务：8次测试验证
   - 复杂任务：20次测试验证
   - 使用Write创建测试文件而非临时脚本

4. **零错误执行**
   - 工具调用成功率100%
   - 没有agent错误
   - 没有需要recover的情况

### 4.2 搜索模式

**最有效的前5步模式**:

**Django-10914** (最高效):
```
步骤1: TodoWrite
步骤2: Grep('FILE_UPLOAD_PERMISSIONS'), Grep('class FileSystemStorage'), Glob('**/storage/*.py')
步骤3: Glob('**/files/storage/*.py'), Grep('FileSystemStorage')
步骤4: bash(find命令定位文件)
步骤5: Read(3个相关文件)
```

**Astropy-14995** (中等):
```
步骤1: Grep('class NDDataRef'), Grep('def multiply')
步骤2: Glob('**/nddata/**/*.py')
步骤3: Read(nddata.py)
步骤4: Grep('class NDDataRef')
步骤5: Glob('**/nddata/*.py')
```

**Astropy-12907** (探索型):
```
步骤1: Glob('**/separable.py'), Glob('**/models/**/*compound*.py')
步骤2: Read(separable.py), Glob('**/models/core.py')
步骤3: Glob('**/core.py')
步骤4: Read(core.py)
步骤5: Grep('_calculate_separability_matrix'), Grep('class CompoundModel')
```

### 4.3 搜索到修改的路径

| 任务 | 首次修改步骤 | Grep次数 | Glob次数 | Read次数 | Bash次数 |
|------|-------------|----------|----------|----------|----------|
| django-10914 | 11 | 4 | 4 | 4 | 2 |
| astropy-14995 | 13 | 5 | 2 | 6 | 0 |
| astropy-12907 | 22 | 2 | 5 | 6 | 10 |

**关键发现**:
- 修改前需要6-15次搜索操作（Grep+Glob+Read）
- Read操作次数稳定在4-6次
- 复杂任务需要更多bash测试来理解问题

---

## 五、对比分析：成功vs可能的失败原因

### 5.1 成功任务的共同特征

✅ **搜索阶段充分但不过度**
- Django: 10步定位（68%的时间）
- Astropy-14995: 12步定位（32%的时间）
- Astropy-12907: 21步定位（30%的时间）

✅ **单次修改成功**
- 所有任务都只用1次Edit修改核心代码
- 没有重复修改或回退

✅ **测试验证与复杂度匹配**
- 简单任务：少量或无测试
- 复杂任务：密集测试（20次+）

✅ **使用正确的模型**
- 所有任务使用glm-4.7模型
- 该模型在代码理解和修改上表现优秀

### 5.2 推测：失败任务可能的问题

❌ **搜索不充分就修改**
- 在前5-10步就尝试修改代码
- 没有充分理解代码结构

❌ **多次修改回退**
- Edit失败后反复尝试
- 没有在修改前充分验证

❌ **测试验证不足**
- 修改后不运行测试
- 不创建测试文件验证

❌ **工具使用错误**
- 工具调用失败率高
- 不处理错误继续执行

---

## 六、最佳实践建议

### 6.1 执行流程建议

**Phase 1: 理解问题** (前10-20%步骤)
1. 使用TodoWrite创建任务列表（可选但推荐）
2. Grep搜索关键类名、函数名、错误信息
3. Glob定位相关文件路径

**Phase 2: 定位代码** (30-50%步骤)
1. Read读取候选文件
2. 使用Bash运行复现脚本
3. 深入理解代码逻辑
4. 再次Grep/Glob确认修改位置

**Phase 3: 修改代码** (1次Edit)
1. 使用Edit进行精准修改
2. 避免创建临时文件或workaround

**Phase 4: 验证修复** (20-50%步骤)
1. 创建测试文件（Write）验证修复
2. 多次运行测试确保正确
3. 使用git diff确认修改
4. 调用task_done完成任务

### 6.2 工具使用建议

**搜索工具组合**:
```
Grep: 用于搜索代码内容（类名、函数名、关键词）
Glob: 用于定位文件路径（模糊匹配）
Read: 用于读取文件内容（确认后）
Bash: 用于运行复现脚本、测试、git命令
```

**修改工具选择**:
```
Edit: 修改现有源代码文件（必须用这个）
Write: 创建测试文件、验证脚本（不要修改源码）
```

**验证工具**:
```
Bash(pytest/python): 运行测试
Bash(git diff): 确认修改
task_done: 完成任务
```

### 6.3 效率优化建议

1. **并行搜索**: 在同一步使用多个Grep/Glob加速定位
   - 例如：`Grep('pattern1'), Grep('pattern2'), Glob('**/*.py')`

2. **批量读取**: 一次Read多个相关文件
   - 例如：`Read(file1), Read(file2), Read(file3)`

3. **早期TodoWrite**: 对复杂任务，第1步创建任务列表帮助规划

4. **测试驱动**: 先创建测试文件，再修改代码

---

## 七、关键指标基准

基于3个成功案例，建立以下基准指标：

| 指标 | 简单任务 | 中等任务 | 复杂任务 |
|------|---------|---------|---------|
| 总步数 | 10-20 | 30-50 | 60-80 |
| 执行时间 | 1-2分钟 | 3-5分钟 | 6-10分钟 |
| 修改前步数 | 8-12 | 12-18 | 20-30 |
| 测试次数 | 0-2 | 5-10 | 15-25 |
| Token消耗 | <500K | 1-2M | 2-3M |
| 工具成功率 | 100% | 100% | 100% |

---

## 八、结论

### 8.1 核心成功因素

1. **充分的代码理解** - 占用30-50%的步骤，但确保修改正确
2. **精准的单次修改** - 所有任务都是1次Edit成功
3. **零错误执行** - 工具调用100%成功率
4. **适度的测试验证** - 根据复杂度调整测试密度

### 8.2 成功公式

```
成功 = 充分搜索（30-50%步骤）
     + 精准修改（1次Edit）
     + 充分测试（20-50%步骤）
     + 零工具错误
```

### 8.3 模型选择

- **GLM-4.7** 在这3个任务中表现优秀
- 关键优势：代码理解、精准定位、工具使用准确

### 8.4 后续优化方向

1. **提高搜索效率**: 减少定位步骤，从11-22步优化到8-15步
2. **测试自动化**: 自动生成测试用例，减少手动验证
3. **知识复用**: 记录成功模式，加速后续类似任务
4. **并行执行**: 同时执行多个独立搜索操作

---

## 附录：文件路径

- Django-10914: `/Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval/swebench_runs/django__django-10914/trajectory_django__django-10914.json`
- Astropy-14995: `/Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval/swebench_runs/astropy__astropy-14995/trajectory_astropy__astropy-14995.json`
- Astropy-12907: `/Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval/swebench_runs/astropy__astropy-12907/trajectory_astropy__astropy-12907.json`

生成时间: 2025-12-25
模型: Claude Opus 4.5
分析者: Sage Agent

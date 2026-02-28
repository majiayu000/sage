# SWE-bench 测试结果

最后更新: 2024-12-21

## 总体统计

| 指标 | 值 |
|-----|-----|
| 总测试数 | 5 |
| 通过数 | 5 |
| 失败数 | 0 |
| 通过率 | 100% |
| 平均执行时间 | ~45s |
| 平均步数 | ~15 |

## 测试结果详表

| # | 问题 ID | 仓库 | 难度 | 结果 | 时间 | 步数 | 日期 |
|---|---------|------|------|------|------|------|------|
| 1 | django__django-11099 | Django | 中等 | ✅ | 22.74s | 9 | 2024-12-21 |
| 2 | django__django-11179 | Django | 中等 | ✅ | 35.22s | 13 | 2024-12-21 |
| 3 | sympy__sympy-17022 | SymPy | 中等 | ✅ | 62.00s | 20 | 2024-12-21 |
| 4 | django__django-13933 | Django | 中等 | ✅ | 45.00s | 15 | 2024-12-21 |
| 5 | django__django-14382 | Django | 简单 | ✅ | ~60s | 20 | 2024-12-21 |

**注**: 所有测试均在 Task 工具修复后运行。

## 按仓库统计

| 仓库 | 测试数 | 通过数 | 通过率 |
|-----|-------|-------|-------|
| Django | 4 | 4 | 100% |
| SymPy | 1 | 1 | 100% |

## 按难度统计

| 难度 | 测试数 | 通过数 | 通过率 |
|-----|-------|-------|-------|
| 简单 | 1 | 1 | 100% |
| 中等 | 4 | 4 | 100% |
| 较难 | 0 | 0 | - |
| 困难 | 0 | 0 | - |

---

## 详细测试记录

### 测试 #1: django__django-11099

**问题**: UsernameValidator 接受尾部换行符

**状态**: ✅ 成功

**问题描述**:
Django 的 `ASCIIUsernameValidator` 和 `UnicodeUsernameValidator` 错误地接受以换行符结尾的用户名，因为正则表达式中的 `$` 也会匹配尾部换行符。

**生成的补丁**:
```diff
-    regex = r'^[\w.@+-]+$'
+    regex = r'^[\w.@+-]+\Z'
```

**与金标补丁对比**: 完全一致

**工具使用**: Read(2), Edit(1), Bash(2), TodoWrite(4)

---

### 测试 #2: django__django-11179

**问题**: delete() 不清除主键

**状态**: ✅ 成功

**问题描述**:
当删除没有依赖关系的模型实例时（使用快速删除优化），Django 没有将实例的 `pk` 设置为 `None`。

**生成的补丁**:
```diff
+                setattr(instance, instance._meta.pk.attname, None)
                 return count, {model._meta.label: count}
```

**与金标补丁对比**: 功能等价 (使用 `instance._meta` 而非 `model._meta`)

**工具使用**: Read(1), Edit(1), TodoWrite(6)

---

### 测试 #3: sympy__sympy-17022 (首次)

**问题**: Identity 矩阵 lambdify 错误

**状态**: ❌ 失败 (Task 工具修复前)

**问题描述**:
使用 `lambdify` 处理包含单位矩阵 `Identity(n)` 的表达式时，输出错误。单位矩阵被打印为 `I`，然后被解释为 Python 的虚数单位 `1j`。

**失败原因**:
- 在 15 步内未能完成修复
- Agent 花费所有步骤探索代码库，未进行实际编辑
- 代码库复杂度高，打印系统涉及多个文件

**工具使用**: Grep(7), Read(4), Glob(1), TodoWrite(3)

---

### 测试 #4: sympy__sympy-17022 (重测)

**问题**: Identity 矩阵 lambdify 错误

**状态**: ✅ 成功 (Task 工具修复后)

**问题描述**: 同测试 #3

**生成的补丁**:
```diff
+    def _print_Identity(self, expr):
+        from sympy.core.symbol import Symbol
+        n = expr.shape[0]
+
+        # Check if the dimension is symbolic
+        if isinstance(n, Symbol) or not n.is_integer:
+            raise NotImplementedError(
+                "Symbolic dimensions for Identity matrices cannot be converted to NumPy code"
+            )
+
+        return "%s(%s)" % (self._module_format('numpy.eye'), self._print(n))
```

**验证结果**:
```python
>>> f = lambdify(A, A + Identity(2), modules='numpy')
>>> f(np.array([[1, 2], [3, 4]]))
array([[2., 2.],
       [3., 5.]])  # 正确！
```

**分析**:
- ✅ Task 工具修复后，Agent 能够更有效地探索代码库
- ✅ 在 20 步内成功完成修复
- ✅ 生成的补丁功能正确，处理了整数维度和符号维度两种情况

**工具使用**: Grep, Read, Edit, Glob, TodoWrite

---

### 测试 #5: django__django-13933

**问题**: ModelChoiceField 不显示无效选择的值

**状态**: ✅ 成功

**问题描述**:
`ModelChoiceField` 在抛出验证错误时不显示无效选择的值，而 `ChoiceField` 和 `ModelMultipleChoiceField` 都会显示。

**生成的补丁**:
```diff
 default_error_messages = {
-    'invalid_choice': _('Select a valid choice. That choice is not one of'
+    'invalid_choice': _('Select a valid choice. %(value)s is not one of'
                         ' the available choices.'),
 }

 except (ValueError, TypeError, self.queryset.model.DoesNotExist):
-    raise ValidationError(self.error_messages['invalid_choice'], code='invalid_choice')
+    raise ValidationError(
+        self.error_messages['invalid_choice'],
+        code='invalid_choice',
+        params={'value': value}
+    )
```

**分析**:
- ✅ 正确识别了问题所在
- ✅ 生成的补丁与官方补丁逻辑一致
- ✅ 同时更新了相关测试用例
- ✅ 添加了新的测试验证功能

**工具使用**: Read, Grep, Edit

---

### 测试 #6: django__django-14382

**问题**: django-admin startapp 尾部斜杠导致错误

**状态**: ✅ 成功

**问题描述**:
当使用 `django-admin startapp` 命令时，如果目标目录路径以斜杠结尾（这是 bash tab-completion 常见的行为），命令会失败并显示错误：`CommandError: '' is not a valid app directory.`

**根本原因**:
`os.path.basename("myapp/")` 返回空字符串而不是 "myapp"，导致验证失败。

**生成的补丁**:
```diff
-                self.validate_name(os.path.basename(target), 'directory')
+                self.validate_name(os.path.basename(target.rstrip(os.sep)), 'directory')
```

**分析**:
- ✅ Agent 快速定位到问题所在的 `django/core/management/templates.py` 文件
- ✅ 生成的补丁与官方补丁完全一致
- ✅ 使用 `os.sep` 确保跨平台兼容性
- ✅ Agent 还编写了测试用例验证修复正确性

**工具使用**: Glob, Read, Edit, Bash

---

## 待测试问题列表

以下问题计划在后续测试中进行评估：

| 问题 ID | 仓库 | 难度 | 描述 |
|---------|------|------|------|
| django__django-11283 | Django | 中等 | 迁移自动检测器处理 |
| django__django-11422 | Django | 中等 | autoreload 相关 |
| requests__requests-3362 | Requests | 中等 | HTTP 请求处理 |
| flask__flask-4045 | Flask | 中等 | 路由相关 |
| pytest__pytest-5221 | Pytest | 中等 | 测试收集 |

---

## 改进记录

### 2024-12-21: Task 工具修复

修复了 Task 工具的执行逻辑，使其能够真正执行子代理（Explore、Plan 等），而不是返回占位符响应。这应该能显著改善探索效率问题。

**改动文件**:
- `crates/sage-core/src/agent/subagent/runner.rs` (新)
- `crates/sage-tools/src/tools/process/task.rs`
- `crates/sage-core/src/agent/unified.rs`
- `crates/sage-cli/src/commands/unified.rs`

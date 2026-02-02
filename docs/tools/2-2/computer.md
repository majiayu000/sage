# Computer Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Computer |
| Claude Code 版本 | 2.0.71 |
| 类别 | 浏览器自动化 |
| Sage 实现 | 未实现 (有 BrowserTool 替代) |

## 功能描述

使用鼠标和键盘与 Web 浏览器交互，并截取屏幕截图。

## 完整 Prompt

```markdown
Use a mouse and keyboard to interact with a web browser, and take screenshots. If you don't have a valid tab ID, use tabs_context_mcp first to get available tabs.
* Whenever you intend to click on an element like an icon, you should consult a screenshot to determine the coordinates of the element before moving the cursor.
* If you tried clicking on a program or link but it failed to load, even after waiting, try adjusting your click location so that the tip of the cursor visually falls on the element that you want to click.
* Make sure to click any buttons, links, icons, etc with the cursor tip in the center of the element. Don't click boxes on their edges unless asked.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| action | string | ✅ | 操作类型 |
| coordinate | array | ❌ | [x, y] 坐标 |
| text | string | ❌ | 输入文本 |
| tab_id | string | ❌ | 浏览器标签 ID |

## 支持的操作 (action)

| 操作 | 说明 |
|------|------|
| screenshot | 截取屏幕截图 |
| click | 点击指定坐标 |
| type | 输入文本 |
| scroll | 滚动页面 |
| key | 按键操作 |
| move | 移动鼠标 |
| drag | 拖拽操作 |

## 设计原理

### 1. 坐标定位
**为什么**:
- 精确控制点击位置
- 适应动态页面布局
- 支持任意 UI 元素

### 2. 先截图再操作
**为什么**:
- 确定元素位置
- 验证页面状态
- 避免盲目点击

### 3. 点击中心位置
**为什么**:
- 提高点击成功率
- 避免边缘误触
- 更可靠的交互

### 4. Tab ID 管理
**为什么**:
- 支持多标签操作
- 明确操作目标
- 避免标签混淆

## 使用场景

### ✅ 应该使用
- 自动化 Web 测试
- 填写表单
- 导航网页
- 截取页面截图

### ❌ 不应该使用
- 简单的 HTTP 请求 (使用 WebFetch)
- API 调用 (使用 Bash curl)
- 文件下载

## 工作流程

```
1. 获取 tab ID (tabs_context_mcp)
   ↓
2. 截取屏幕截图 (screenshot)
   ↓
3. 分析截图确定坐标
   ↓
4. 执行操作 (click/type/scroll)
   ↓
5. 验证结果 (screenshot)
```

## Sage 实现状态

Sage 有 `BrowserTool` 作为替代，基于 Playwright：
- 支持无头浏览器
- 元素选择器定位
- 更高级的自动化功能
- 不依赖坐标定位

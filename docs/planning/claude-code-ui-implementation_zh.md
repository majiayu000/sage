# Claude Code UI 实现详解

> 基于 open-claude-code v2.0.76 源码分析

## 目录

1. [渲染架构](#1-渲染架构)
2. [布局系统](#2-布局系统)
3. [终端处理](#3-终端处理)
4. [输入处理](#4-输入处理)
5. [状态管理](#5-状态管理)
6. [进度和状态显示](#6-进度和状态显示)
7. [核心组件](#7-核心组件)
8. [与 rnk 的对比](#8-与-rnk-的对比)

---

## 1. 渲染架构

### 1.1 Ink (React for CLI) 框架

**文件位置**: `src_v2.0.76/cli.readable.js` (lines 185100-187700)

Claude Code 使用 **Ink** - 基于 React 的终端 UI 框架。

**组件统计** (来自 `UI_COMPONENTS.md`):
- React Components: **206**
- Ink Components: **15** (Text, Box, Spacer, Static, Newline, Transform)
- Color Definitions: **386**
- Keyboard Shortcuts: **37**
- Message Formats: **3198**

**核心 Ink 组件**:
```javascript
// Ink primitives
Text     // 文本显示，支持样式
Box      // Flexbox 布局容器
Spacer   // 弹性空间填充
Static   // 不重新渲染的静态内容
Newline  // 换行
Transform // 文本转换
```

### 1.2 主渲染入口

**文件**: `cli.readable.js` (lines 187609-187627)

```javascript
render(A) {
  this.currentNode = A;
  let installRAL = lWB.default.createElement(
    KeA,  // App 组件包装器
    {
      initialTheme: this.options.theme,
      stdin: this.options.stdin,
      stdout: this.options.stdout,
      stderr: this.options.stderr,
      exitOnCtrlC: this.options.exitOnCtrlC,
      onExit: this.unmount,
      ink2: this.options.ink2,          // Ink2 模式标志
      terminalColumns: this.terminalColumns,
      terminalRows: this.terminalRows,
    },
    A,  // 子内容
  );
  Mi.updateContainer(installRAL, this.container, null, rf);
}
```

### 1.3 Context Provider 层级

**文件**: `cli.readable.js` (lines 187009-187063)

应用使用多层 React Context Providers:

```javascript
render() {
  return Kv.default.createElement(
    nNA.Provider,  // 终端尺寸 context
    {
      value: {
        columns: this.props.terminalColumns,
        rows: this.props.terminalRows,
      },
    },
    Kv.default.createElement(
      nU.Provider,  // Ink2 特性标志
      { value: this.props.ink2 },
      Kv.default.createElement(
        ltA.Provider,  // 退出处理器 context
        { value: { exit: this.handleExit } },
        Kv.default.createElement(
          Nl1,  // 主题 provider
          { initialState: this.props.initialTheme },
          Kv.default.createElement(
            itA.Provider,  // Stdin context
            {
              value: {
                stdin: this.props.stdin,
                setRawMode: this.handleSetRawMode,
                isRawModeSupported: this.isRawModeSupported(),
                internal_exitOnCtrlC: this.props.exitOnCtrlC,
                internal_eventEmitter: this.internal_eventEmitter,
              },
            },
            Kv.default.createElement(
              ntA.Provider,  // 焦点管理 context
              {
                value: {
                  activeId: this.state.activeFocusId,
                  add: this.addFocusable,
                  remove: this.removeFocusable,
                  activate: this.activateFocusable,
                  deactivate: this.deactivateFocusable,
                  enableFocus: this.enableFocus,
                  disableFocus: this.disableFocus,
                  focusNext: this.focusNext,
                  focusPrevious: this.focusPrevious,
                  focus: this.focus,
                },
              },
              // 错误处理或子内容
              this.state.error
                ? Kv.default.createElement(Rl1, { error: this.state.error })
                : this.props.children,
            ),
          ),
        ),
      ),
    ),
  );
}
```

**Context 层级结构**:
```
nNA.Provider (终端尺寸)
└── nU.Provider (Ink2 特性)
    └── ltA.Provider (退出处理)
        └── Nl1 (主题)
            └── itA.Provider (Stdin/Raw Mode)
                └── ntA.Provider (焦点管理)
                    └── children
```

---

## 2. 布局系统

### 2.1 Flexbox 使用

**来自 UI_COMPONENTS.md**:

```markdown
## Layout Properties

### flexDirection
- `row`
- `column`

### alignItems
- `flex-start`
- `flex-end`
- `center`

### justifyContent
- `space-between`
- `flex-end`
- `flex-start`
- `center`
```

**示例组件布局** (lines 463727-463798):

```javascript
a8.createElement(
  T,  // Box 组件
  { flexDirection: "column", width: "100%" },
  // 分割线
  Z && a8.createElement($8, { dividerColor: "permission", dividerDimColor: !1 }),
  a8.createElement(
    T,
    { flexDirection: "column", paddingX: Z ? 1 : 0 },
    a8.createElement(
      T,
      { flexDirection: "column" },
      a8.createElement(
        T,
        { marginBottom: 1, flexDirection: "column" },
        a8.createElement(C, { color: "remember", bold: !0 }, "Select model"),
        a8.createElement(
          C,
          { dimColor: !0 },
          "Switch between Claude models...",
        ),
      ),
    ),
  ),
);
```

### 2.2 Yoga 布局引擎

**文件**: `cli.readable.js` (lines 187504-187508)

```javascript
(this.rootNode.onComputeLayout = () => {
  if (this.isUnmounted) return;
  if (this.rootNode.yogaNode)
    (this.rootNode.yogaNode.setWidth(this.terminalColumns),
      this.rootNode.yogaNode.calculateLayout(void 0, void 0, m1A.LTR));
});
```

- 使用 **Yoga** (Facebook 的跨平台布局引擎)
- Flexbox 计算，LTR (Left-to-Right) 方向
- 与 Web CSS Flexbox 语义一致

### 2.3 固定底部布局实现

**关键策略**:

1. **根容器**: `height: 100%` + `flexDirection: column`
2. **内容区域**: `flexGrow: 1` (占据剩余空间)
3. **底部元素**: 固定高度 (自然被推到底部)

```javascript
// 固定底部布局模式
Box({
  flexDirection: "column",
  height: "100%",
  children: [
    // 内容区域 - 占据所有剩余空间
    Box({ flexGrow: 1, overflow: "hidden", children: [...messages] }),
    // 分割线
    Divider(),
    // 输入区域 - 固定高度
    InputLine(),
    // 状态栏 - 固定高度
    StatusBar(),
  ]
});
```

### 2.4 渲染输出系统

**文件**: `cli.readable.js` (lines 185157-185221)

渲染系统有多种模式:

```javascript
class zl1 {
  render(A, installRAL) {
    return this.options.ink2
      ? this.render_v2(A, installRAL)
      : this.render_v1(A, installRAL);
  }

  render_v1(A, installRAL) {
    if (this.options.debug) return this.getRenderOpsDebug_DEPRECATED(installRAL);
    if (!this.options.isTTY)
      return [{ type: "stdout", content: installRAL.staticOutput }];

    let B = nIB(A, installRAL);  // 检查 resize/offscreen
    if (B) return this.getRenderOpsForAllOutput_CAUSES_FLICKER(installRAL, B);

    // ... 高效渲染
    return updateProgressState(Z, A, installRAL);
  }

  renderEfficiently(A, installRAL) {
    let B = installRAL.output + "\n";
    if (B === this.state.previousOutput) return [];  // 无变化

    let G = this.state.previousOutput
      ? getLineCount(this.state.previousOutput, A.columns)
      : 0;
    this.state.previousOutput = B;

    let Z = [];
    // 处理光标可见性
    if (!installRAL.cursorVisible && A.cursorVisible)
      Z.push({ type: "cursorHide" });
    else if (installRAL.cursorVisible && !A.cursorVisible)
      Z.push({ type: "cursorShow" });

    return (
      Z.push({ type: "clear", count: G }),  // 清除之前的行
      Z.push({ type: "stdout", content: installRAL.output }),
      Z.push({ type: "stdout", content: "\n" }),
      Z
    );
  }
}
```

**渲染操作类型**:
- `stdout`: 输出内容
- `clear`: 清除 N 行
- `cursorHide` / `cursorShow`: 光标控制
- `clearTerminal`: 清屏 (会触发 flicker)

---

## 3. 终端处理

### 3.1 ANSI 转义码

**文件**: `cli.readable.js` (lines 167802-167827)

```javascript
function cursorUp(A = 1) {
  return A === 0 ? "" : createCsiSequence(A, "A");
}
function cursorDown(A = 1) {
  return A === 0 ? "" : createCsiSequence(A, "B");
}
function cursorForward(A = 1) {
  return A === 0 ? "" : createCsiSequence(A, "C");
}
function cursorBack(A = 1) {
  return A === 0 ? "" : createCsiSequence(A, "D");
}

function moveCursor(A, installRAL) {
  let B = "";
  if (A < 0) B += cursorBack(-A);
  else if (A > 0) B += cursorForward(A);
  if (installRAL < 0) B += cursorUp(-installRAL);
  else if (installRAL > 0) B += cursorDown(installRAL);
  return B;
}

function eraseLines(A) {
  if (A <= 0) return "";
  let installRAL = "";
  for (let B = 0; B < A; B++)
    if (((installRAL += to8), B < A - 1))
      installRAL += cursorUp(1);
  return ((installRAL += so8), installRAL);
}
```

### 3.2 CSI 控制码定义

**文件**: `cli.readable.js` (lines 167840-167867)

```javascript
mJ = {
  CUU: 65,  // Cursor Up - \x1b[nA
  CUD: 66,  // Cursor Down - \x1b[nB
  CUF: 67,  // Cursor Forward - \x1b[nC
  CUB: 68,  // Cursor Back - \x1b[nD
  CNL: 69,  // Cursor Next Line - \x1b[nE
  CPL: 70,  // Cursor Previous Line - \x1b[nF
  CHA: 71,  // Cursor Horizontal Absolute - \x1b[nG
  CUP: 72,  // Cursor Position - \x1b[n;mH
  CHT: 73,  // Cursor Horizontal Tab - \x1b[nI
  VPA: 100, // Vertical Position Absolute - \x1b[nd
  HVP: 102, // Horizontal Vertical Position - \x1b[n;mf
  ED: 74,   // Erase in Display - \x1b[nJ
  EL: 75,   // Erase in Line - \x1b[nK
  ECH: 88,  // Erase Character - \x1b[nX
  SGR: 109, // Select Graphic Rendition - \x1b[n;...m
  DSR: 110, // Device Status Report - \x1b[6n
  DECSCUSR: 113, // Set Cursor Style - \x1b[n q
  DECSTBM: 114,  // Set Top and Bottom Margins - \x1b[n;mr
};
```

**常用 ANSI 序列**:

| 序列 | 功能 | 代码 |
|------|------|------|
| `\x1b[H` | 光标归位 | cursor_home() |
| `\x1b[nA` | 光标上移 n 行 | cursorUp(n) |
| `\x1b[nB` | 光标下移 n 行 | cursorDown(n) |
| `\x1b[2K` | 清除整行 | eraseLine() |
| `\x1b[K` | 清除到行尾 | eraseEndOfLine() |
| `\x1b[2J` | 清屏 | eraseScreen() |
| `\x1b[?25l` | 隐藏光标 | hideCursor() |
| `\x1b[?25h` | 显示光标 | showCursor() |
| `\x1b[?1049h` | 进入备用屏幕 | enterAltScreen() |
| `\x1b[?1049l` | 离开备用屏幕 | leaveAltScreen() |

### 3.3 终端环境检测

**文件**: `src/ui/terminal/p1_z1_m1_n1.js` (lines 22-65)

```javascript
function detectTerminalEnvironment() {
  // IDE 检测
  if (process.env.CURSOR_TRACE_ID) return "cursor";
  if (process.env.VSCODE_GIT_ASKPASS_MAIN?.includes("/.cursor-server/")) return "cursor";
  if (process.env.VSCODE_GIT_ASKPASS_MAIN?.includes("/.windsurf-server/")) return "windsurf";

  let addItem4 = process.env.__CFBundleIdentifier?.toLowerCase();
  if (addItem4?.includes("vscodium")) return "codium";
  if (addItem4?.includes("windsurf")) return "windsurf";
  if (addItem4?.includes("com.google.android.studio")) return "androidstudio";

  // JetBrains IDEs
  if (addItem4) {
    for (let config of jetbrainsIdeList)
      if (addItem4.includes(config)) return config;
  }

  // Visual Studio
  if (process.env.VisualStudioVersion) return "visualstudio";
  if (process.env.TERMINAL_EMULATOR === "JetBrains-JediTerm") return "pycharm";

  // 终端模拟器
  if (process.env.TERM === "xterm-ghostty") return "ghostty";
  if (process.env.TERM?.includes("kitty")) return "kitty";
  if (process.env.TERM_PROGRAM) return process.env.TERM_PROGRAM;
  if (process.env.STY) return "screen";
  if (process.env.KONSOLE_VERSION) return "konsole";
  if (process.env.GNOME_TERMINAL_SERVICE) return "gnome-terminal";
  if (process.env.XTERM_VERSION) return "xterm";
  if (process.env.VTE_VERSION) return "vte-based";
  if (process.env.TERMINATOR_UUID) return "terminator";
  if (process.env.KITTY_WINDOW_ID) return "kitty";
  if (process.env.ALACRITTY_LOG) return "alacritty";
  if (process.env.TILIX_ID) return "tilix";
  if (process.env.WT_SESSION) return "windows-terminal";

  if (!process.stdout.isTTY) return "non-interactive";
  return null;
}
```

**支持的终端** (20+):
- 编辑器: Cursor, VS Code, Codium, Windsurf, Android Studio, Visual Studio, JetBrains 系列
- 终端: iTerm2, Ghostty, Kitty, Alacritty, Hyper, Wezterm
- Linux: GNOME Terminal, Konsole, Terminator, Tilix
- Windows: Windows Terminal
- 远程: Screen, Tmux

### 3.4 Raw Mode 管理

**文件**: `cli.readable.js` (lines 187077-187100)

```javascript
handleSetRawMode = (A) => {
  let { stdin: installRAL } = this.props;
  if (!this.isRawModeSupported())
    if (installRAL === process.stdin)
      throw Error(`Raw mode is not supported on the current process.stdin...`);
    else
      throw Error(`Raw mode is not supported on the stdin provided to Ink...`);

  if ((installRAL.setEncoding("utf8"), A)) {
    // 启用 Raw Mode
    if (this.rawModeEnabledCount === 0)
      (installRAL.ref(),
        installRAL.setRawMode(true),
        installRAL.addListener("readable", this.handleReadable),
        this.props.stdout.write(bWB));  // 启用鼠标追踪
    this.rawModeEnabledCount++;
    return;
  }

  // 禁用 Raw Mode
  if (--this.rawModeEnabledCount === 0)
    (this.props.stdout.write(fWB),  // 禁用鼠标追踪
      installRAL.setRawMode(false),
      installRAL.removeListener("readable", this.handleReadable),
      installRAL.unref());
};
```

**Raw Mode 特点**:
- 禁用行缓冲，每个按键立即可读
- 禁用回显
- 禁用信号处理 (Ctrl+C 需要手动处理)
- 引用计数管理，支持多组件共享

### 3.5 Resize 处理

**文件**: `cli.readable.js` (lines 187539-187546)

```javascript
handleResize = () => {
  if (
    ((this.terminalColumns = this.options.stdout.columns || 80),
    (this.terminalRows = this.options.stdout.rows || 24),
    this.currentNode !== null)
  )
    this.render(this.currentNode);  // 尺寸变化时重新渲染
};
```

### 3.6 屏幕状态结构

**文件**: `cli.readable.js` (lines 185057-185106)

```javascript
// Screen state structure
{
  output: "",           // 当前输出字符串
  outputHeight: 0,      // 输出高度（行数）
  staticOutput: "",     // 静态内容（不重新渲染）
  rows: A,              // 终端行数
  columns: installRAL,  // 终端列数
  cursorVisible: true,  // 光标可见性
  screen: createScreen(0, 0, B),  // 屏幕缓冲区
  viewport: { width: 0, height: 0 },
  cursor: { x: 0, y: 0, visible: true },
  progress: B,          // 进度条状态
}
```

---

## 4. 输入处理

### 4.1 键盘快捷键

**来自 UI_COMPONENTS.md**:

```javascript
const keyboardShortcuts = [
  "alt+p", "alt+t", "backspace",
  "cmd+a", "ctrl+a", "ctrl+b", "ctrl+g", "ctrl+o", "ctrl+r", "ctrl+s", "ctrl+t", "ctrl+v",
  "delete", "down", "end", "enter", "escape",
  "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
  "home", "left", "pagedown", "pageup", "right", "space", "tab", "up"
];
```

### 4.2 按键解析

**文件**: `cli.readable.js` (lines 187101-187134)

```javascript
// 带超时的按键解析（处理不完整的转义序列）
processInput = (A) => {
  let [installRAL, B] = parseKeyPress(this.keyParseState, A);
  if (((this.keyParseState = B), installRAL.length > 0))
    Mi.discreteUpdates(kt8, this, installRAL, void 0, void 0);

  if (this.keyParseState.incomplete) {
    if (this.incompleteEscapeTimer)
      clearTimeout(this.incompleteEscapeTimer);
    this.incompleteEscapeTimer = setTimeout(
      this.flushIncomplete,
      this.keyParseState.mode === "IN_PASTE"
        ? this.PASTE_TIMEOUT   // 500ms - 粘贴超时
        : this.NORMAL_TIMEOUT, // 50ms - 普通超时
    );
  }
};

handleInput = (A) => {
  // Ctrl+C 退出
  if (A === "\x03" && this.props.exitOnCtrlC) this.handleExit();
  // Ctrl+Z 挂起
  if (A === "\x1a" && vt8) this.handleSuspend();
  // Tab/Shift+Tab 焦点导航
  if (this.state.isFocusEnabled && this.state.focusables.length > 0) {
    if (A === St8) this.focusNext();
    if (A === xt8) this.focusPrevious();
  }
};
```

**超时策略**:
- 普通输入: 50ms
- 粘贴模式: 500ms
- 用于区分真正的 Escape 键和转义序列

### 4.3 焦点管理

**文件**: `cli.readable.js` (lines 187156-187210)

```javascript
enableFocus = () => {
  this.setState({ isFocusEnabled: true });
};

disableFocus = () => {
  this.setState({ isFocusEnabled: false });
};

focus = (A) => {
  this.setState((installRAL) => {
    if (!installRAL.focusables.some((G) => G?.id === A)) return installRAL;
    return { activeFocusId: A };
  });
};

focusNext = () => {
  this.setState((A) => {
    let installRAL = A.focusables.find((G) => G.isActive)?.id;
    return { activeFocusId: this.findNextFocusable(A) ?? installRAL };
  });
};

focusPrevious = () => {
  this.setState((A) => {
    let installRAL = A.focusables.findLast((G) => G.isActive)?.id;
    return { activeFocusId: this.findPreviousFocusable(A) ?? installRAL };
  });
};

addFocusable = (A, { autoFocus: installRAL }) => {
  this.setState((B) => {
    let G = B.activeFocusId;
    if (!G && installRAL) G = A;
    return {
      activeFocusId: G,
      focusables: [...B.focusables, { id: A, isActive: true }],
    };
  });
};
```

---

## 5. 状态管理

### 5.1 useAppState Hook

**文件**: `cli.readable.js` (various locations)

```javascript
let [B, G] = useAppState();
// B = 当前状态对象
// G = setState 函数

// 状态结构示例
{
  notifications: [],
  mainLoopModel: "claude-opus-4.5",
  mainLoopModelForSession: null,
  tasks: [],
  toolPermissionContext: {},
  statusLineText: "",
  queuedCommands: [],
  promptSuggestionEnabled: true,
  showExpandedTodos: false,
  promptSuggestion: {
    text: null,
    promptId: null,
    shownAt: 0,
    acceptedAt: 0,
    generationRequestId: null,
  },
  promptCoaching: {
    tip: null,
  },
  terminalProgressBarEnabled: true,
}
```

### 5.2 懒加载初始化模式

**文件**: `cli.readable.js` (throughout)

```javascript
// 组件使用懒加载进行模块初始化
var createLazyInitializer = (A, installRAL) => () =>
  (A && (installRAL = A((A = 0))), installRAL);

// 使用示例
var Nt2 = createLazyInitializer(() => {
  initializeInkComponents();
  initializeStripAnsi();
  initializeTerminalInput();
  initializeVimMode();
  initializeModelSelector();
  initializeFileAccessPermissions();
  initializeThemeContext();
  initializeTelemetry();
  initializeImageUtils();
  initializeKeyboardShortcuts();
  // ... more initializations
  r2 = createModuleWrapper(React(), 1);
});
```

---

## 6. 进度和状态显示

### 6.1 终端进度条

**文件**: `cli.readable.js` (line 460368-460371, 487715-487721)

```javascript
// 进度条配置
{
  id: "terminalProgressBarEnabled",
  label: "Terminal progress bar",
  value: X.terminalProgressBarEnabled,
  type: "boolean",
}

// 进度条渲染
function UkA({ state: A, percentage: installRAL }) {
  if (!b1().terminalProgressBarEnabled) return null;
  return fU0.createElement(zeA, { state: A, percentage: installRAL });
}

// 渲染输出中的进度状态追踪
function lIB(A) {
  if (A.nodeName === "ink-progress") {
    let installRAL = A.attributes.state;
    if (installRAL) return { state: installRAL, percentage: A.attributes.percentage };
  }
  // 递归检查子节点
  for (let installRAL of A.childNodes)
    if ("nodeName" in installRAL && installRAL.nodeName !== "#text") {
      let B = lIB(installRAL);
      if (B) return B;
    }
  return;
}
```

### 6.2 闪烁防止

**文件**: `cli.readable.js` (lines 187553-187574)

```javascript
onRender() {
  if (this.isUnmounted || this.isPaused) return;
  let A = this.options.stdout.rows || 24,
    installRAL = this.options.stdout.columns || 80,
    B = this.renderer({
      terminalWidth: installRAL,
      terminalRows: A,
      isTTY: this.options.stdout.isTTY,
      ink2: this.options.ink2,
      prevScreen: this.prevFrame.screen,
    }),
    G = this.log.render(this.prevFrame, B);
  this.prevFrame = B;

  // 检测闪烁条件
  for (let Z of G)
    if (Z.type === "clearTerminal")
      this.options.onFlicker?.(
        B.outputHeight,
        B.rows,
        this.options.ink2,
        Z.reason,
      );

  Sl1(this.terminal, xl1(G));  // 写入终端
}
```

**闪烁防止策略**:
1. 差异更新 - 只更新变化的行
2. 光标管理 - 渲染时隐藏光标
3. 缓冲输出 - 批量写入终端
4. 事件回调 - 闪烁发生时通知

---

## 7. 核心组件

### 7.1 组件命名

**命名组件** (来自 UI_COMPONENTS.md):
- `IdeSelectionIndicatorComponent` - IDE 选择指示器
- `SearchInput` - 搜索输入框
- `RuleSourceLabel` - 规则来源标签
- `SearchableRuleList` - 可搜索规则列表
- `PressEnterToContinue` - 继续提示
- `ShortcutHint` - 快捷键提示

**核心原语** (混淆后的名称):
- `T` - Box 组件 (flexbox 容器)
- `C` - Text 组件
- `x0` - Select/下拉组件
- `$8` - Divider 组件
- `uB` - 水平布局 (Fragment 包装器)

### 7.2 颜色系统

**来自 UI_COMPONENTS.md**: 386 种颜色定义

**语义颜色**:
- `"permission"` - 权限对话框
- `"remember"` - 记住/设置
- `"error"` - 错误消息
- `dimColor` - 暗色/次要文本

### 7.3 图标和 Emoji

**来自 UI_COMPONENTS.md**:
```
✅ ❌ ↓ ← ↑ → ■ ● ▲ ▼ ✔ ⚠ ✘ ◆ ◇ □ ◄ ► ○ ✓ ✗
```

### 7.4 权限模式

Claude Code 有三种权限模式，通过 Shift+Tab 循环:

| 模式 | 显示文本 | 行为 |
|------|----------|------|
| Normal | `permissions required` | 每个危险操作需要确认 |
| Bypass | `bypass permissions on` | 跳过权限确认 |
| Plan | `plan mode` | 只规划不执行 |

---

## 8. 与 rnk 的对比

### 8.1 框架对比

| 特性 | Claude Code (Ink) | Sage (rnk) |
|------|-------------------|------------|
| 语言 | JavaScript/TypeScript | Rust |
| 框架基础 | React | 自定义 React-like |
| 布局引擎 | Yoga (C++) | Taffy (Rust) |
| 渲染模式 | 全屏/Inline 双模式 | App/println 双模式 |
| 组件系统 | JSX + React Hooks | 声明式 Builder API |
| 状态管理 | useAppState Hook | use_signal Hook |
| 事件处理 | React 事件系统 | use_input Hook |

### 8.2 固定底部布局对比

**Claude Code 实现**:
```javascript
Box({
  flexDirection: "column",
  height: "100%",
  children: [
    Box({ flexGrow: 1, children: messages }),  // 内容区
    Divider(),                                  // 分割线
    InputLine(),                                // 输入行
    StatusBar(),                                // 状态栏
  ]
});
```

**rnk 实现**:
```rust
Box::new()
    .flex_direction(FlexDirection::Column)
    .height(Dimension::Percent(100.0))
    .child(
        Box::new()
            .flex_grow(1.0)
            .overflow_y(Overflow::Hidden)
            .child(messages)
            .into_element(),
    )
    .child(render_separator(width))
    .child(render_input_line(input))
    .child(render_status_bar(mode))
    .into_element()
```

### 8.3 关键差异

1. **Inline 模式处理**
   - Claude Code: 清除 + 重写策略，每次渲染清除之前的输出
   - rnk: 增量更新策略，只更新变化的行

2. **Viewport 感知**
   - Claude Code: 始终使用终端完整高度
   - rnk: 现在已修复为 inline 模式也使用完整终端高度

3. **进度条**
   - Claude Code: 使用 `ink-progress` 自定义节点
   - rnk: 可通过组件或 Static 元素实现

---

## 总结

Claude Code 的 UI 架构基于:

1. **Ink 框架** - React 驱动的终端渲染，使用 Yoga 布局
2. **Context Providers** - 多层嵌套的 Context 管理主题、输入、焦点、尺寸
3. **高效渲染** - 差异更新、光标管理、闪烁防止
4. **Raw Mode 输入** - 直接键盘处理，超时驱动的序列解析
5. **ANSI 序列** - 完整的 CSI 转义码终端控制
6. **终端检测** - 环境感知渲染，支持 20+ 终端类型
7. **状态管理** - 自定义 `useAppState()` hook 配合 React 模式
8. **懒加载** - 按需模块初始化

rnk 已经实现了大部分核心功能，通过最近的修复（inline 模式使用完整终端高度），现在可以正确支持 Claude Code 风格的固定底部布局。

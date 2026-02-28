# Crush UI Analysis

## Overview

Crush is a Charmbracelet AI assistant built on Bubbletea. It demonstrates advanced patterns for building sophisticated terminal AI interfaces, including dialog systems, command management, and template-based prompts.

---

## 1. Architecture Overview

### 1.1 Core Stack

- **Bubbletea** - Application framework (Elm architecture)
- **Lipgloss** - Styling
- **Glamour** - Markdown rendering
- **Bubbles** - UI components (textinput, list, viewport)
- **Chroma** - Syntax highlighting

### 1.2 Model Structure

```go
type Model struct {
    // Core state
    messages     []Message
    input        textinput.Model
    viewport     viewport.Model

    // UI state
    width        int
    height       int
    focused      component

    // Dialogs
    dialogStack  *DialogStack

    // Commands
    commands     *CommandRegistry

    // LLM
    client       llm.Client
    streaming    bool
}
```

---

## 2. Dialog System

### 2.1 Dialog Stack

Crush implements a dialog stack for modal overlays:

```go
type DialogStack struct {
    dialogs []Dialog
    mu      sync.RWMutex
}

type Dialog interface {
    ID() string
    Update(msg tea.Msg) (Dialog, tea.Cmd)
    View() string
    Width() int
    Height() int
}

func (s *DialogStack) Push(d Dialog) {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.dialogs = append(s.dialogs, d)
}

func (s *DialogStack) Pop() Dialog {
    s.mu.Lock()
    defer s.mu.Unlock()

    if len(s.dialogs) == 0 {
        return nil
    }

    d := s.dialogs[len(s.dialogs)-1]
    s.dialogs = s.dialogs[:len(s.dialogs)-1]
    return d
}

func (s *DialogStack) HasDialogs() bool {
    s.mu.RLock()
    defer s.mu.RUnlock()
    return len(s.dialogs) > 0
}
```

### 2.2 Dialog Types

**Command Picker Dialog:**
```go
type CommandPickerDialog struct {
    list     list.Model
    commands []Command
    filter   string
}

func (d *CommandPickerDialog) Update(msg tea.Msg) (Dialog, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "enter":
            selected := d.list.SelectedItem().(commandItem)
            return nil, executeCommand(selected.command)
        case "esc":
            return nil, closeDialog
        }
    }

    var cmd tea.Cmd
    d.list, cmd = d.list.Update(msg)
    return d, cmd
}
```

**Confirmation Dialog:**
```go
type ConfirmDialog struct {
    title    string
    message  string
    onConfirm tea.Cmd
    focused  int // 0 = yes, 1 = no
}

func (d *ConfirmDialog) View() string {
    buttons := []string{"Yes", "No"}
    for i, btn := range buttons {
        if i == d.focused {
            buttons[i] = selectedStyle.Render("[" + btn + "]")
        } else {
            buttons[i] = normalStyle.Render(" " + btn + " ")
        }
    }

    return borderStyle.Render(
        lipgloss.JoinVertical(lipgloss.Center,
            titleStyle.Render(d.title),
            d.message,
            lipgloss.JoinHorizontal(lipgloss.Center, buttons...),
        ),
    )
}
```

### 2.3 Compositing Dialogs

```go
func (m Model) View() string {
    // Render base view
    base := m.renderMainView()

    // Overlay dialogs
    if m.dialogStack.HasDialogs() {
        dialog := m.dialogStack.Current()

        // Calculate center position
        x := (m.width - dialog.Width()) / 2
        y := (m.height - dialog.Height()) / 2

        // Render dialog on top
        return lipgloss.Place(
            m.width, m.height,
            lipgloss.Center, lipgloss.Center,
            dialog.View(),
            lipgloss.WithWhitespaceChars(" "),
            lipgloss.WithWhitespaceForeground(lipgloss.Color("0")),
        )
    }

    return base
}
```

---

## 3. Command System

### 3.1 Command Categories

```go
const (
    CategorySystem = "system"  // Built-in commands
    CategoryUser   = "user"    // User-defined commands
    CategoryMCP    = "mcp"     // MCP server commands
)

type Command struct {
    Name        string
    Description string
    Category    string
    Aliases     []string
    Template    string      // Markdown template
    Args        []ArgDef    // Argument definitions
    Execute     CommandFunc // Direct execution
}

type ArgDef struct {
    Name        string
    Required    bool
    Description string
    Default     string
}
```

### 3.2 Command Discovery

Commands are discovered from directories:

```go
func discoverCommands(dirs []string) []Command {
    var commands []Command

    for _, dir := range dirs {
        // Walk directory for .md and .tpl files
        filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
            if err != nil {
                return err
            }

            ext := filepath.Ext(path)
            if ext == ".md" || ext == ".tpl" {
                cmd, err := parseCommandFile(path)
                if err == nil {
                    commands = append(commands, cmd)
                }
            }
            return nil
        })
    }

    return commands
}
```

### 3.3 Template Format

Commands are defined in Markdown with frontmatter:

```markdown
---
name: analyze
description: Analyze code for issues
category: user
args:
  - name: file
    required: true
    description: File to analyze
  - name: depth
    required: false
    default: "3"
---

# Code Analysis

Please analyze the following file: $FILE

Consider:
- Code quality
- Potential bugs
- Performance issues

Depth of analysis: $DEPTH
```

### 3.4 Variable Substitution

```go
func (c *Command) Render(args map[string]string) string {
    result := c.Template

    // Replace $VARIABLE with values
    for _, arg := range c.Args {
        placeholder := "$" + strings.ToUpper(arg.Name)
        value := args[arg.Name]
        if value == "" {
            value = arg.Default
        }
        result = strings.ReplaceAll(result, placeholder, value)
    }

    return result
}
```

---

## 4. SKILL.md Standard

Crush follows the Agent Skills open standard:

### 4.1 SKILL.md Format

```markdown
---
name: code-review
version: 1.0.0
description: Expert code review assistant
author: Charmbracelet
license: MIT
triggers:
  - /review
  - /code-review
context:
  - "*.go"
  - "*.rs"
---

# Code Review Expert

You are an expert code reviewer. When the user asks for a code review:

1. Analyze the code structure
2. Check for best practices
3. Identify potential issues
4. Suggest improvements

## Guidelines

- Be constructive
- Explain reasoning
- Provide examples
```

### 4.2 Skill Loading

```go
type Skill struct {
    Name        string
    Version     string
    Description string
    Author      string
    Triggers    []string
    Context     []string
    Prompt      string
}

func loadSkill(path string) (*Skill, error) {
    content, err := os.ReadFile(path)
    if err != nil {
        return nil, err
    }

    // Parse frontmatter and content
    skill := &Skill{}
    // ... parsing logic

    return skill, nil
}
```

---

## 5. TOOL.md Pattern

Each tool has a companion description file:

### 5.1 TOOL.md Format

```markdown
---
name: file_edit
description: Edit files with precise changes
---

# File Edit Tool

The file_edit tool allows making precise edits to files.

## Usage

Provide:
- `path`: The file path to edit
- `old_content`: The exact content to replace
- `new_content`: The replacement content

## Examples

To replace a function:

```json
{
  "path": "main.go",
  "old_content": "func old() {}",
  "new_content": "func new() {}"
}
```

## Notes

- Content must match exactly
- Use for surgical edits
```

### 5.2 Tool Injection

```go
func buildSystemPrompt(tools []Tool) string {
    var toolDocs []string

    for _, tool := range tools {
        // Read TOOL.md for each tool
        docPath := filepath.Join(tool.Dir, "TOOL.md")
        if content, err := os.ReadFile(docPath); err == nil {
            toolDocs = append(toolDocs, string(content))
        }
    }

    return basePrompt + "\n\n# Available Tools\n\n" +
           strings.Join(toolDocs, "\n\n---\n\n")
}
```

---

## 6. Message Rendering

### 6.1 Message Types

```go
type Message struct {
    Role      string
    Content   string
    Timestamp time.Time
    Thinking  string  // Thinking content
    ToolCalls []ToolCall
}

type ToolCall struct {
    Name   string
    Input  string
    Output string
    Status string // pending, running, complete, error
}
```

### 6.2 Message Rendering

```go
func renderMessage(m Message, width int) string {
    var parts []string

    // Role indicator
    switch m.Role {
    case "user":
        parts = append(parts, userStyle.Render("❯ "))
    case "assistant":
        parts = append(parts, assistantStyle.Render("● "))
    }

    // Content with markdown
    rendered, _ := glamour.Render(m.Content, "dark")
    parts = append(parts, rendered)

    // Tool calls
    for _, tc := range m.ToolCalls {
        parts = append(parts, renderToolCall(tc, width))
    }

    // Thinking (collapsed/expanded)
    if m.Thinking != "" {
        parts = append(parts, renderThinking(m.Thinking))
    }

    return lipgloss.JoinVertical(lipgloss.Left, parts...)
}
```

### 6.3 Thinking Display

```go
type thinkingState int

const (
    thinkingCollapsed thinkingState = iota
    thinkingExpanded
)

func renderThinking(content string, state thinkingState) string {
    if state == thinkingCollapsed {
        return dimStyle.Render("∴ Thinking... (ctrl+o to expand)")
    }

    return thinkingStyle.Render(
        lipgloss.JoinVertical(lipgloss.Left,
            "∴ Thinking:",
            content,
        ),
    )
}
```

---

## 7. Layout Structure

### 7.1 Fixed-Bottom Layout

```go
func (m Model) View() string {
    // Calculate heights
    statusHeight := 1
    inputHeight := 1
    separatorHeight := 1
    contentHeight := m.height - statusHeight - inputHeight - separatorHeight*2

    // Build layout
    return lipgloss.JoinVertical(lipgloss.Left,
        // Scrollable content
        m.renderMessages(contentHeight),
        // Separator
        m.renderSeparator(),
        // Input
        m.input.View(),
        // Separator
        m.renderSeparator(),
        // Status bar
        m.renderStatusBar(),
    )
}

func (m Model) renderSeparator() string {
    return dimStyle.Render(strings.Repeat("─", m.width))
}

func (m Model) renderStatusBar() string {
    mode := m.getModeText()
    scroll := m.getScrollIndicator()

    return lipgloss.JoinHorizontal(lipgloss.Left,
        accentStyle.Render("▸▸ "),
        dimStyle.Render(mode),
        dimStyle.Render(" "),
        dimStyle.Render(scroll),
    )
}
```

### 7.2 Scroll Handling

```go
func (m *Model) updateViewport() {
    // Set viewport content
    content := m.renderAllMessages()
    m.viewport.SetContent(content)

    // Auto-scroll to bottom on new messages
    if m.autoScroll {
        m.viewport.GotoBottom()
    }
}

func (m Model) getScrollIndicator() string {
    if m.viewport.TotalLineCount() <= m.viewport.Height {
        return ""
    }

    percent := m.viewport.ScrollPercent() * 100
    return fmt.Sprintf("[%3.0f%%]", percent)
}
```

---

## 8. Input Handling

### 8.1 Input Modes

```go
type inputMode int

const (
    modeNormal inputMode = iota
    modeVim
    modeEmacs
)

func (m Model) handleInput(msg tea.KeyMsg) (Model, tea.Cmd) {
    switch m.inputMode {
    case modeVim:
        return m.handleVimInput(msg)
    case modeEmacs:
        return m.handleEmacsInput(msg)
    default:
        return m.handleNormalInput(msg)
    }
}
```

### 8.2 Keyboard Shortcuts

```go
var shortcuts = map[string]func(*Model) tea.Cmd{
    "ctrl+c":     func(m *Model) tea.Cmd { return tea.Quit },
    "ctrl+l":     func(m *Model) tea.Cmd { return tea.ClearScreen },
    "ctrl+o":     func(m *Model) tea.Cmd { return m.toggleThinking() },
    "shift+tab":  func(m *Model) tea.Cmd { return m.cycleMode() },
    "ctrl+/":     func(m *Model) tea.Cmd { return m.showHelp() },
    "/":          func(m *Model) tea.Cmd { return m.showCommandPicker() },
}

func (m Model) handleGlobalShortcuts(msg tea.KeyMsg) (Model, tea.Cmd) {
    key := msg.String()
    if fn, ok := shortcuts[key]; ok {
        return m, fn(&m)
    }
    return m, nil
}
```

---

## 9. Streaming Support

### 9.1 Stream Handling

```go
type streamChunk struct {
    Content  string
    Thinking string
    Done     bool
}

func (m Model) handleStream(msg streamChunk) (Model, tea.Cmd) {
    if msg.Done {
        m.streaming = false
        return m, nil
    }

    // Append to current message
    if len(m.messages) > 0 {
        last := &m.messages[len(m.messages)-1]
        last.Content += msg.Content
        if msg.Thinking != "" {
            last.Thinking += msg.Thinking
        }
    }

    // Update viewport
    m.updateViewport()

    return m, nil
}
```

### 9.2 Cancellation

```go
func (m Model) cancelStream() tea.Cmd {
    return func() tea.Msg {
        m.streamCancel()
        return streamCancelledMsg{}
    }
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        if msg.String() == "esc" && m.streaming {
            return m, m.cancelStream()
        }
    }
    // ...
}
```

---

## 10. Key Patterns Summary

### Architectural Patterns

1. **Dialog Stack** - Modal overlays with push/pop semantics
2. **Command Registry** - Extensible command system with discovery
3. **Template Prompts** - Markdown-based prompt templates
4. **SKILL.md/TOOL.md** - Standard formats for skills and tools

### UI Patterns

1. **Fixed-bottom layout** - Manual height calculation in View()
2. **Viewport scrolling** - bubbles/viewport for content
3. **Thinking collapse** - Toggle with keyboard shortcut
4. **Status bar** - Mode indicator + scroll position

### Input Patterns

1. **Global shortcuts** - Map-based shortcut registry
2. **Mode cycling** - shift+tab to cycle modes
3. **Stream cancellation** - ESC to cancel
4. **Vim/Emacs modes** - Multiple input modes

### Performance Patterns

1. **Viewport-based rendering** - Only render visible content
2. **Auto-scroll control** - Toggle auto-scroll on manual scroll
3. **Throttled updates** - Batch rapid stream updates

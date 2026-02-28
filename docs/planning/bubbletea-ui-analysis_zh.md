# Bubbletea TUI Framework Analysis

## Overview

Bubbletea is a Go framework for building terminal applications using the Elm Architecture (Model-Update-View). It's widely used in the Go ecosystem (Charm tools, Gum, etc.) and powers applications like Crush.

---

## 1. Core Architecture: The Elm Architecture

### 1.1 Model-Update-View Pattern

```go
// The Model holds all application state
type Model struct {
    choices  []string
    cursor   int
    selected map[int]struct{}
}

// Init returns initial commands to run
func (m Model) Init() tea.Cmd {
    return nil
}

// Update handles messages and updates state
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "q", "ctrl+c":
            return m, tea.Quit
        case "up", "k":
            if m.cursor > 0 {
                m.cursor--
            }
        case "down", "j":
            if m.cursor < len(m.choices)-1 {
                m.cursor++
            }
        case "enter", " ":
            _, ok := m.selected[m.cursor]
            if ok {
                delete(m.selected, m.cursor)
            } else {
                m.selected[m.cursor] = struct{}{}
            }
        }
    }
    return m, nil
}

// View renders the UI as a string
func (m Model) View() string {
    s := "What should we buy?\n\n"

    for i, choice := range m.choices {
        cursor := " "
        if m.cursor == i {
            cursor = ">"
        }

        checked := " "
        if _, ok := m.selected[i]; ok {
            checked = "x"
        }

        s += fmt.Sprintf("%s [%s] %s\n", cursor, checked, choice)
    }

    s += "\nPress q to quit.\n"
    return s
}
```

### 1.2 Message Types

All events are typed messages:

```go
// Built-in message types
tea.KeyMsg          // Keyboard input
tea.MouseMsg        // Mouse events
tea.WindowSizeMsg   // Terminal resize

// Custom message types
type tickMsg time.Time
type errMsg struct{ err error }
type dataMsg struct{ data []byte }
```

### 1.3 Commands

Commands are functions that perform I/O and return messages:

```go
// Command signature
type Cmd func() Msg

// Built-in commands
tea.Quit           // Exit the program
tea.ClearScreen    // Clear terminal
tea.EnterAltScreen // Enter alternate screen
tea.ExitAltScreen  // Exit alternate screen
tea.Batch(...)     // Run multiple commands

// Custom command
func tickCmd() tea.Cmd {
    return tea.Tick(time.Second, func(t time.Time) tea.Msg {
        return tickMsg(t)
    })
}

// Async command
func fetchData(url string) tea.Cmd {
    return func() tea.Msg {
        resp, err := http.Get(url)
        if err != nil {
            return errMsg{err}
        }
        defer resp.Body.Close()
        data, _ := io.ReadAll(resp.Body)
        return dataMsg{data}
    }
}
```

---

## 2. Bubbles: Reusable Components

Bubbletea has an ecosystem of reusable components called "Bubbles":

### 2.1 Text Input

```go
import "github.com/charmbracelet/bubbles/textinput"

type model struct {
    textInput textinput.Model
}

func initialModel() model {
    ti := textinput.New()
    ti.Placeholder = "Enter text..."
    ti.Focus()
    ti.CharLimit = 156
    ti.Width = 20

    return model{textInput: ti}
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.textInput, cmd = m.textInput.Update(msg)
    return m, cmd
}

func (m model) View() string {
    return m.textInput.View()
}
```

### 2.2 List Component

```go
import "github.com/charmbracelet/bubbles/list"

type item struct {
    title, desc string
}

func (i item) Title() string       { return i.title }
func (i item) Description() string { return i.desc }
func (i item) FilterValue() string { return i.title }

type model struct {
    list list.Model
}

func initialModel() model {
    items := []list.Item{
        item{title: "Raspberry Pi", desc: "A tiny computer"},
        item{title: "Arduino", desc: "Open-source electronics"},
    }

    l := list.New(items, list.NewDefaultDelegate(), 0, 0)
    l.Title = "My Items"
    l.SetShowStatusBar(false)
    l.SetFilteringEnabled(true)

    return model{list: l}
}
```

### 2.3 Viewport (Scrolling)

```go
import "github.com/charmbracelet/bubbles/viewport"

type model struct {
    viewport viewport.Model
    content  string
}

func initialModel() model {
    vp := viewport.New(80, 20)
    vp.SetContent(longContent)

    return model{viewport: vp, content: longContent}
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.viewport, cmd = m.viewport.Update(msg)
    return m, cmd
}

func (m model) View() string {
    return m.viewport.View()
}
```

### 2.4 Progress Bar

```go
import "github.com/charmbracelet/bubbles/progress"

type model struct {
    progress progress.Model
    percent  float64
}

func initialModel() model {
    return model{
        progress: progress.New(progress.WithDefaultGradient()),
    }
}

func (m model) View() string {
    return m.progress.ViewAs(m.percent)
}
```

### 2.5 Spinner

```go
import "github.com/charmbracelet/bubbles/spinner"

type model struct {
    spinner spinner.Model
}

func initialModel() model {
    s := spinner.New()
    s.Spinner = spinner.Dot
    s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

    return model{spinner: s}
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case spinner.TickMsg:
        var cmd tea.Cmd
        m.spinner, cmd = m.spinner.Update(msg)
        return m, cmd
    }
    return m, nil
}
```

---

## 3. Lipgloss: Styling Library

Bubbletea uses Lipgloss for styling (not layout):

```go
import "github.com/charmbracelet/lipgloss"

// Define styles
var (
    titleStyle = lipgloss.NewStyle().
        Bold(true).
        Foreground(lipgloss.Color("#FAFAFA")).
        Background(lipgloss.Color("#7D56F4")).
        Padding(0, 1)

    itemStyle = lipgloss.NewStyle().
        PaddingLeft(4)

    selectedItemStyle = lipgloss.NewStyle().
        PaddingLeft(2).
        Foreground(lipgloss.Color("170"))

    borderStyle = lipgloss.NewStyle().
        Border(lipgloss.RoundedBorder()).
        BorderForeground(lipgloss.Color("62")).
        Padding(1, 2)
)

// Use styles
func (m model) View() string {
    s := titleStyle.Render("My App")
    s += "\n"

    for i, item := range m.items {
        if i == m.cursor {
            s += selectedItemStyle.Render("> " + item)
        } else {
            s += itemStyle.Render(item)
        }
        s += "\n"
    }

    return borderStyle.Render(s)
}
```

### 3.1 Layout with Lipgloss

```go
// Horizontal layout
lipgloss.JoinHorizontal(lipgloss.Top,
    leftColumn,
    rightColumn,
)

// Vertical layout
lipgloss.JoinVertical(lipgloss.Left,
    header,
    content,
    footer,
)

// Centering
lipgloss.Place(width, height,
    lipgloss.Center, lipgloss.Center,
    content,
)
```

---

## 4. Program Options

### 4.1 Basic Usage

```go
func main() {
    p := tea.NewProgram(initialModel())

    if _, err := p.Run(); err != nil {
        fmt.Printf("Error: %v", err)
        os.Exit(1)
    }
}
```

### 4.2 Program Options

```go
p := tea.NewProgram(
    initialModel(),
    tea.WithAltScreen(),        // Use alternate screen
    tea.WithMouseCellMotion(),  // Enable mouse tracking
    tea.WithMouseAllMotion(),   // Track all mouse motion
    tea.WithoutCatchPanics(),   // Don't recover panics
    tea.WithInput(reader),      // Custom input
    tea.WithOutput(writer),     // Custom output
)
```

### 4.3 Sending Messages from Outside

```go
// Get a channel to send messages
p := tea.NewProgram(model)

go func() {
    // Send message from goroutine
    p.Send(myCustomMsg{})
}()

p.Run()
```

---

## 5. Terminal Handling

### 5.1 Termenv Integration

Bubbletea uses termenv for terminal capabilities:

```go
import "github.com/muesli/termenv"

// Detect color profile
profile := termenv.ColorProfile()

// Check terminal features
output := termenv.NewOutput(os.Stdout)
if output.HasDarkBackground() {
    // Use light colors
}
```

### 5.2 Alternate Screen

```go
// Enter alternate screen in Init
func (m model) Init() tea.Cmd {
    return tea.EnterAltScreen
}

// Exit on quit
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    if key, ok := msg.(tea.KeyMsg); ok && key.String() == "q" {
        return m, tea.Sequence(
            tea.ExitAltScreen,
            tea.Quit,
        )
    }
    return m, nil
}
```

### 5.3 Window Size Handling

```go
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.WindowSizeMsg:
        m.width = msg.Width
        m.height = msg.Height
        // Update viewport/list dimensions
        m.viewport.Width = msg.Width
        m.viewport.Height = msg.Height - headerHeight - footerHeight
    }
    return m, nil
}
```

---

## 6. Patterns from Crush

### 6.1 Dialog Stack Pattern

```go
type dialogStack struct {
    dialogs []Dialog
}

type Dialog interface {
    ID() string
    Update(msg tea.Msg) (Dialog, tea.Cmd)
    View() string
}

func (s *dialogStack) Push(d Dialog) {
    s.dialogs = append(s.dialogs, d)
}

func (s *dialogStack) Pop() Dialog {
    if len(s.dialogs) == 0 {
        return nil
    }
    d := s.dialogs[len(s.dialogs)-1]
    s.dialogs = s.dialogs[:len(s.dialogs)-1]
    return d
}

func (s *dialogStack) Current() Dialog {
    if len(s.dialogs) == 0 {
        return nil
    }
    return s.dialogs[len(s.dialogs)-1]
}
```

### 6.2 Command System Pattern

```go
type Command struct {
    Name        string
    Description string
    Category    string
    Template    string
    Execute     func(args []string) tea.Cmd
}

type CommandRegistry struct {
    commands map[string]Command
}

func (r *CommandRegistry) Register(cmd Command) {
    r.commands[cmd.Name] = cmd
}

func (r *CommandRegistry) Execute(name string, args []string) tea.Cmd {
    if cmd, ok := r.commands[name]; ok {
        return cmd.Execute(args)
    }
    return nil
}
```

### 6.3 Template-Based Prompts

```go
// Commands loaded from .md or .tpl files
type TemplateCommand struct {
    Name     string
    Template string // Markdown template with $VARIABLES
}

func (t *TemplateCommand) Render(vars map[string]string) string {
    result := t.Template
    for k, v := range vars {
        result = strings.ReplaceAll(result, "$"+k, v)
    }
    return result
}
```

---

## 7. Performance Patterns

### 7.1 Batched Updates

```go
// Combine multiple commands
return m, tea.Batch(
    cmd1,
    cmd2,
    cmd3,
)
```

### 7.2 Throttled Rendering

```go
type model struct {
    lastRender time.Time
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    // Throttle rapid updates
    if time.Since(m.lastRender) < 16*time.Millisecond {
        return m, nil
    }
    m.lastRender = time.Now()
    // ... handle update
    return m, nil
}
```

### 7.3 Lazy Loading

```go
type model struct {
    items    []item
    loaded   int
    pageSize int
}

func (m model) loadMore() tea.Cmd {
    return func() tea.Msg {
        // Load next page
        newItems := fetchItems(m.loaded, m.pageSize)
        return itemsLoadedMsg{items: newItems}
    }
}
```

---

## 8. Key Differences from React-based Frameworks

| Aspect | Bubbletea | Ink/React |
|--------|-----------|-----------|
| State Updates | Immutable, returns new model | Mutable via hooks |
| Side Effects | Commands (tea.Cmd) | useEffect |
| Layout | Manual string concatenation + Lipgloss | Yoga flexbox |
| Components | Interfaces + composition | React components |
| Async | Commands return messages | Promises/async-await |
| Type Safety | Go's type system | TypeScript |

---

## 9. API Summary

### Core Types

```go
// The main model interface
type Model interface {
    Init() Cmd
    Update(Msg) (Model, Cmd)
    View() string
}

// Messages
type Msg interface{}
type KeyMsg Key
type MouseMsg Mouse
type WindowSizeMsg struct{ Width, Height int }

// Commands
type Cmd func() Msg
```

### Program Options

```go
tea.WithAltScreen()         // Alternate screen buffer
tea.WithMouseCellMotion()   // Mouse click tracking
tea.WithMouseAllMotion()    // Full mouse motion
tea.WithInput(io.Reader)    // Custom input
tea.WithOutput(io.Writer)   // Custom output
tea.WithFilter(func)        // Message filter
```

### Built-in Commands

```go
tea.Quit                    // Exit program
tea.Batch(cmds...)          // Run commands concurrently
tea.Sequence(cmds...)       // Run commands sequentially
tea.EnterAltScreen          // Enter alt screen
tea.ExitAltScreen           // Exit alt screen
tea.ClearScreen             // Clear screen
tea.DisableMouse            // Disable mouse
tea.Tick(duration, func)    // Timer tick
```

### Bubbles Components

- `textinput` - Single-line text input
- `textarea` - Multi-line text input
- `list` - Filterable list with delegation
- `table` - Table with headers
- `viewport` - Scrollable content area
- `spinner` - Loading indicator
- `progress` - Progress bar
- `paginator` - Page navigation
- `filepicker` - File selection
- `help` - Key binding help
- `key` - Key binding definitions
- `stopwatch` - Timer
- `timer` - Countdown timer

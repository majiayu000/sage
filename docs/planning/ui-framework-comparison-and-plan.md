# UI Framework Comparison and Improvement Plan

Based on comprehensive analysis of rnk, Ink, Bubbletea, and Crush frameworks.

---

## 1. Feature Comparison Matrix

| Feature | rnk (Rust) | Ink (React/Node) | Bubbletea (Go) | Sage Current |
|---------|------------|------------------|----------------|--------------|
| **Framework Type** | Declarative | Declarative (React) | Model-Update-View | Mixed (rnk + ANSI) |
| **Layout Engine** | Taffy (Rust) | Yoga (C++) | lipgloss | Taffy |
| **Flexbox Support** | Full | Full | Limited | Full |
| **Component Model** | Builder pattern | JSX + Hooks | Interface-based | Builder pattern |
| **State Management** | use_signal hook | React useState | tea.Model | watch::channel |
| **Input Handling** | use_input hook | useInput hook | tea.Msg | crossterm raw mode |
| **Rendering Mode** | App/println dual | Fullscreen/Inline dual | Fullscreen | println only |
| **Static Output** | Static component | Static component | Not built-in | Not used |
| **Scroll Support** | ScrollableBox, virtual_scroll | Built-in | viewport package | Not implemented |
| **Progress/Spinner** | Spinner component | ora/spinners | spinner bubble | Thread-based spinner |
| **Focus Management** | Not built-in | Built-in | Built-in (bubbles) | Not implemented |
| **Message Components** | Message, ToolCall, ThinkingBlock | Custom | Not built-in | Custom render_* fns |
| **Terminal Detection** | Not built-in | 20+ terminals | termenv | Not implemented |
| **Flicker Prevention** | Diff-based update | Diff-based update | Built-in | Basic |
| **Raw Mode Handling** | crossterm | Node.js TTY | Built-in | crossterm |
| **Context/Provider** | Not built-in | React Context layers | Not built-in | Not needed |

---

## 2. Gap Analysis

### 2.1 rnk vs Ink

| Missing in rnk | Impact | Priority |
|----------------|--------|----------|
| Focus management | Can't do multi-input forms | Medium |
| Terminal detection | No adaptive rendering | Low |
| Keyboard shortcut registry | Manual key handling | Medium |
| Progress bar with state | Limited progress UI | Low |
| useStdout/useStdin hooks | Less flexibility | Low |
| Error boundary | No graceful error UI | Low |

### 2.2 rnk vs Bubbletea

| Missing in rnk | Impact | Priority |
|----------------|--------|----------|
| Model-Update-View pattern | Different paradigm (not a gap) | N/A |
| Composable bubbles | Component reuse patterns | Medium |
| Built-in input components | Must build from scratch | Medium |
| Command pattern (tea.Cmd) | Different async handling | N/A |

### 2.3 Sage-Specific Gaps

| Issue | Current State | Target State |
|-------|---------------|--------------|
| Rendering Mode | println only | App mode with fixed-bottom |
| State Management | EventAdapter not fully used | Proper hook integration |
| Spinner | Thread-based, manual | rnk Spinner component |
| Message Components | Custom render_* functions | rnk Message components |
| Scroll | Not implemented | use_scroll hook |

---

## 3. Best Practices from Each Framework

### 3.1 From Ink (Claude Code)

1. **Context Provider Architecture**
   - Multiple nested contexts for different concerns
   - Enables clean separation and testability

2. **Static + Dynamic Split**
   - Historical content marked as Static (not re-rendered)
   - Active content updated via diff
   - Prevents flicker and improves performance

3. **Flicker Prevention**
   - Hide cursor during render
   - Batch writes to terminal
   - Diff-based updates (only changed lines)

4. **Terminal Detection**
   - Environment variable detection for 20+ terminals/IDEs
   - Adaptive rendering based on capabilities

5. **Keyboard Sequence Parsing**
   - Timeout-based parsing (50ms normal, 500ms paste)
   - Distinguish real Escape from escape sequences

### 3.2 From Bubbletea (Crush)

1. **Model-Update-View (MUV) Pattern**
   - Clear separation of concerns
   - Predictable state updates
   - Easy to test

2. **Message-Driven Architecture**
   - All events are typed messages
   - Commands return async results as messages
   - Easy to trace event flow

3. **Composable Components**
   - Dialog system with stack management
   - Reusable input components
   - Filter/search built into list

4. **Command System Design**
   - Three tiers: System, User, MCP
   - Markdown template support with $VARIABLES
   - Hierarchical command discovery from directories

5. **Template-Based Prompts**
   - `.tpl` and `.md` files for prompts
   - Runtime variable substitution
   - Easy customization without code changes

### 3.3 From Crush Specifically

1. **SKILL.md Standard**
   - Agent Skills open standard
   - Standardized metadata format
   - Compatible with Claude Code format

2. **TOOL.md Pattern**
   - Each tool has companion description file
   - Injected into system prompt
   - Easier tool documentation

3. **Dialog Stack Management**
   - Push/pop dialog pattern
   - Type-safe dialog models
   - Clean separation from main UI

---

## 4. Improvement Plan for rnk

### 4.1 High Priority

**1. Focus Management System**
```rust
// Proposed API
pub struct FocusManager {
    focusables: Vec<FocusableId>,
    active_id: Option<FocusableId>,
}

pub fn use_focus() -> FocusHandle;
pub fn use_focus_manager() -> FocusManager;
```
- Tab/Shift+Tab navigation
- Auto-focus support
- Focus trap for modals

**2. Enhanced Keyboard Input**
```rust
pub struct KeyboardShortcuts {
    shortcuts: HashMap<KeyBinding, Action>,
}

impl KeyboardShortcuts {
    pub fn register(&mut self, binding: &str, action: impl Fn());
    pub fn handle(&self, key: KeyEvent);
}
```
- Shortcut registry
- Conflict detection
- Context-aware shortcuts

**3. Input Components**
```rust
pub struct TextInput {
    value: String,
    cursor: usize,
    placeholder: Option<String>,
    // vim mode, history, etc.
}

pub struct Select<T> {
    options: Vec<T>,
    selected: usize,
    filter: Option<String>,
}
```

### 4.2 Medium Priority

**4. Dialog Component System**
```rust
pub struct DialogStack {
    dialogs: Vec<Box<dyn Dialog>>,
}

pub trait Dialog {
    fn id(&self) -> DialogId;
    fn update(&mut self, msg: Message) -> Option<DialogResult>;
    fn view(&self) -> Element;
}
```

**5. Terminal Capability Detection**
```rust
pub fn detect_terminal() -> TerminalInfo {
    // Check env vars for IDE/terminal type
    // Return capabilities (colors, unicode, etc.)
}
```

### 4.3 Low Priority

**6. Flicker Prevention Improvements**
- Add onFlicker callback
- Improve diff algorithm
- Cursor hiding during render

**7. Error Boundary Component**
```rust
pub struct ErrorBoundary<F: Fn(Error) -> Element> {
    fallback: F,
}
```

---

## 5. Improvement Plan for Sage CLI

### Phase 1: Fix Current Issues (Immediate)

**1. Use rnk's Message Components Consistently**
- Replace custom `render_*` functions with `Message::user()`, `Message::assistant()`
- Already have `rnk::println(Message::user(...).into_element())`
- Remove duplicate ANSI escape code usage

**2. Proper Spinner Integration**
- Use rnk's `Spinner` component instead of custom thread-based spinner
- Properly integrate with async execution
- Handle ESC cancellation correctly

**3. State Management Cleanup**
- The `EventAdapter` with `watch::channel` is good
- Ensure `set_global_adapter()` is called before any events

### Phase 2: UI Patterns from Claude Code (Short-term)

**4. Fixed Bottom Layout**
```rust
fn app() -> Element {
    fixed_bottom_layout(
        // Scrollable message area
        ScrollableBox::new()
            .scroll_offset_y(scroll.offset_y() as u16)
            .children(messages.iter().map(render_message))
            .into_element(),
        // Bottom area
        Box::new()
            .flex_direction(FlexDirection::Column)
            .child(render_separator())
            .child(render_input_line(&input))
            .child(render_status_bar(&status))
            .into_element(),
    )
}
```

**5. Static Output for History**
- Use `Static` component for completed messages
- Only re-render current/active content
- Improves performance for long conversations

**6. Thinking Display Modes**
- Collapsed: `"∴ Thinking... (ctrl+o to expand)"`
- Expanded: Show full thinking content
- Toggle with Ctrl+O
- Use rnk's `ThinkingBlock` component

### Phase 3: Enhanced Features (Medium-term)

**7. Permission Mode Cycling**
- Normal: `"permissions required"`
- Bypass: `"bypass permissions on"`
- Plan: `"plan mode"`
- Shift+Tab to cycle

**8. Scroll Support**
- Use rnk's `use_scroll` hook
- Virtual scrolling for long message lists
- PageUp/PageDown navigation

**9. Input History**
- Arrow up/down for history navigation
- Store in session state
- Persist across sessions

### Phase 4: Advanced Features (Long-term)

**10. Fullscreen Mode Option**
- `sage --fullscreen` for immersive experience
- Alternate screen buffer
- Mouse support

**11. Slash Command UI**
- Learn from Crush's dialog system
- Category tabs (System, User, MCP)
- Fuzzy search/filter
- Argument input dialog

**12. Multi-pane Layout**
- Sidebar for sessions/files
- Main chat area
- Collapsible panels

---

## 6. Specific Code Changes Needed

### 6.1 sage-cli/src/app.rs

```rust
// Replace run_app() with proper rnk app
pub async fn run_app() -> io::Result<()> {
    // Initialize global adapter FIRST
    let adapter = EventAdapter::with_default_state();
    set_global_adapter(adapter.clone());

    // Create executor
    let executor = create_executor().await?;

    // Run rnk app in fullscreen mode
    rnk::render(App { executor, adapter })
        .fullscreen()
        .run()
}

// Define App component
fn App(props: AppProps) -> Element {
    let state = use_state(|| AppState::default());
    let scroll = use_scroll();

    // Subscribe to adapter updates
    use_effect(|| {
        // Update local state from adapter
    });

    use_input(|input, key| {
        // Handle keyboard shortcuts
    });

    fixed_bottom_layout(
        render_messages(&state.messages, scroll.offset_y()),
        render_bottom_area(&state.input, &state.status),
    )
}
```

### 6.2 rnk Additions

**src/hooks/use_focus.rs:**
```rust
pub struct FocusHandle {
    id: FocusableId,
    is_focused: bool,
}

pub fn use_focus() -> FocusHandle {
    // Register with focus manager
    // Return handle with focus state
}
```

**src/components/text_input.rs:**
```rust
pub struct TextInput {
    value: Signal<String>,
    cursor: Signal<usize>,
    placeholder: String,
}

impl TextInput {
    pub fn new() -> Self { ... }
    pub fn placeholder(mut self, text: &str) -> Self { ... }
    pub fn on_change(mut self, f: impl Fn(&str)) -> Self { ... }
    pub fn into_element(self) -> Element { ... }
}
```

---

## 7. Priority Order

1. **Fix adapter initialization** (immediate)
2. **Use rnk Message components consistently** (immediate)
3. **Implement fixed bottom layout** (short-term)
4. **Add scroll support** (short-term)
5. **Implement permission mode cycling** (medium-term)
6. **Add slash command dialog** (medium-term)
7. **Fullscreen mode option** (long-term)

---

## 8. Metrics for Success

- UI renders correctly without ANSI artifacts
- Smooth scrolling in long conversations
- Keyboard shortcuts work consistently
- No flicker during updates
- Feature parity with Claude Code for core interactions

---

## 9. Summary

### Key Takeaways

1. **rnk is capable** - Has most features needed for Claude Code-like UI
2. **Sage isn't using rnk fully** - Currently only uses `println()` mode, not fullscreen app
3. **Event adapter architecture is good** - Just needs proper initialization
4. **Crush patterns are valuable** - Command system, dialog stack, templates

### Next Steps

1. Add focus management to rnk (new feature)
2. Add TextInput component to rnk (new feature)
3. Update Sage to use fullscreen mode with fixed_bottom_layout
4. Implement thinking toggle with Ctrl+O
5. Add permission mode cycling with Shift+Tab

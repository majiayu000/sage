# rnk TUI Framework Analysis

## Overview

rnk (version 0.6.4) is a React-like declarative terminal UI framework for Rust, inspired by [Ink](https://github.com/vadimdemedes/ink) (JavaScript) and [Bubbletea](https://github.com/charmbracelet/bubbletea) (Go). It provides a component-based architecture with flexbox layout, reactive state management via hooks, and both inline and fullscreen rendering modes.

---

## 1. Rendering Architecture

### 1.1 App Runner and Render Loop

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/app.rs`**

The `App` struct is the main application runner that manages the render loop:

```rust
// Lines 425-443
pub struct App<F>
where
    F: Fn() -> Element,
{
    component: F,
    terminal: Terminal,
    layout_engine: LayoutEngine,
    hook_context: Rc<RefCell<HookContext>>,
    options: AppOptions,
    should_exit: Arc<AtomicBool>,
    needs_render: Arc<AtomicBool>,
    alt_screen_state: Arc<AtomicBool>,
    static_lines: Vec<String>,
    last_width: u16,
    last_height: u16,
}
```

The main render loop runs at a configurable FPS (default 60):

```rust
// Lines 487-558 - App::run()
pub fn run(&mut self) -> std::io::Result<()> {
    // Initialize global state for cross-thread communication
    init_global_state(Arc::clone(&self.needs_render));

    // Enter terminal mode based on options
    if self.options.alternate_screen {
        self.terminal.enter()?;
    } else {
        self.terminal.enter_inline()?;
    }

    let frame_duration = Duration::from_millis(1000 / self.options.fps as u64);
    let mut last_render = Instant::now();

    // Initial render
    self.render_frame()?;

    loop {
        // Handle input
        if let Some(event) = Terminal::poll_event(Duration::from_millis(10))? {
            self.handle_event(event);
        }

        if self.should_exit.load(Ordering::SeqCst) { break; }

        // Check for external render requests (from other threads)
        if take_render_request() {
            self.needs_render.store(true, Ordering::SeqCst);
        }

        // Throttle rendering - only render if needed and time elapsed
        let now = Instant::now();
        let time_elapsed = now.duration_since(last_render) >= frame_duration;
        let render_requested = self.needs_render.load(Ordering::SeqCst);

        if render_requested && time_elapsed {
            self.needs_render.store(false, Ordering::SeqCst);
            self.render_frame()?;
            last_render = now;
        }
    }
    // ...cleanup
}
```

### 1.2 Frame Rendering Process

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/app.rs`, Lines 674-734**

```rust
fn render_frame(&mut self) -> std::io::Result<()> {
    // Clear input and mouse handlers before render
    clear_input_handlers();
    clear_mouse_handlers();

    // Get terminal size
    let (width, height) = Terminal::size()?;

    // Set up app context for use_app hook
    set_app_context(Some(AppContext::new(self.should_exit.clone())));

    // Build element tree with hooks context
    let root = with_hooks(self.hook_context.clone(), || (self.component)());

    // Clear app context after render
    set_app_context(None);

    // Compute layout for dynamic content
    self.layout_engine.compute(&dynamic_root, width, height);

    // Render to output buffer
    let mut output = Output::new(content_width, render_height);
    self.render_element(&dynamic_root, &mut output, 0.0, 0.0);

    // Write to terminal
    self.terminal.render(&output.render())
}
```

### 1.3 Inline vs Fullscreen Mode

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/terminal.rs`**

rnk supports two rendering modes:

**Inline Mode (Default):**
- Output appears at the current cursor position
- Content persists in terminal history
- Supports `println()` for persistent messages

**Fullscreen Mode:**
- Uses alternate screen buffer
- Content is cleared on exit
- Like vim, less, or Bubbletea's `WithAltScreen()`

```rust
// Lines 167-193 - Fullscreen mode entry
pub fn enter(&mut self) -> std::io::Result<()> {
    enable_raw_mode()?;
    self.raw_mode = true;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    self.alternate_screen = true;
    self.cursor_hidden = true;
    Ok(())
}

// Lines 196-208 - Inline mode entry
pub fn enter_inline(&mut self) -> std::io::Result<()> {
    enable_raw_mode()?;
    self.raw_mode = true;
    let mut stdout = stdout();
    write!(stdout, "{}", ansi::hide_cursor())?;
    stdout.flush()?;
    self.cursor_hidden = true;
    self.inline_lines_rendered = 0;
    Ok(())
}
```

### 1.4 Output Buffer

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/output.rs`**

The `Output` struct is a virtual 2D character grid that collects rendered content before writing to the terminal:

```rust
// Lines 84-101
pub struct Output {
    pub width: u16,
    pub height: u16,
    grid: Vec<Vec<StyledChar>>,
    clip_stack: Vec<ClipRegion>,
}

impl Output {
    pub fn new(width: u16, height: u16) -> Self {
        let grid = vec![vec![StyledChar::new(' '); width as usize]; height as usize];
        Self {
            width,
            height,
            grid,
            clip_stack: Vec::new(),
        }
    }
}
```

Each cell stores a styled character:

```rust
// Lines 8-19
pub struct StyledChar {
    pub ch: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub inverse: bool,
}
```

---

## 2. Layout System

### 2.1 Taffy Layout Engine Usage

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/layout/engine.rs`**

rnk uses the [Taffy](https://github.com/DioxusLabs/taffy) layout engine (v0.7) for flexbox calculations:

```rust
// Lines 26-37
pub struct LayoutEngine {
    taffy: TaffyTree<NodeContext>,
    node_map: HashMap<ElementId, NodeId>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            node_map: HashMap::new(),
        }
    }
}
```

Layout computation with custom text measurement:

```rust
// Lines 86-99
pub fn compute(&mut self, root: &Element, width: u16, height: u16) {
    if let Some(root_node) = self.build_tree(root) {
        let _ = self.taffy.compute_layout_with_measure(
            root_node,
            taffy::Size {
                width: AvailableSpace::Definite(width as f32),
                height: AvailableSpace::Definite(height as f32),
            },
            |known_dimensions, available_space, _node_id, node_context, _style| {
                measure_text_node(known_dimensions, available_space, node_context)
            },
        );
    }
}
```

### 2.2 Flexbox Implementation

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/core/style.rs`**

The Style struct provides a complete flexbox implementation:

```rust
// Lines 310-380
pub struct Style {
    // Display
    pub display: Display,

    // Positioning
    pub position: Position,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,

    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: bool,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub justify_content: JustifyContent,

    // Spacing
    pub padding: Edges,
    pub margin: Edges,
    pub gap: f32,
    pub row_gap: Option<f32>,
    pub column_gap: Option<f32>,

    // Size
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,

    // Overflow
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    // ... more properties
}
```

### 2.3 Fixed-Bottom Layout Support

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/components/scrollable.rs`, Lines 210-260**

```rust
/// Create a fixed-bottom layout with scrollable content area
pub fn fixed_bottom_layout(content: Element, bottom: Element) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(crate::core::Dimension::Percent(100.0))
        .child(
            // Scrollable content area takes remaining space
            Box::new()
                .flex_grow(1.0)
                .overflow_y(Overflow::Hidden)
                .child(content)
                .into_element(),
        )
        .child(bottom)
        .into_element()
}
```

The App ensures inline mode uses full terminal height for fixed-bottom layouts:

```rust
// Lines 717-726 in app.rs
let render_height = if self.terminal.is_alt_screen() {
    // Fullscreen mode: use actual content height
    (root_layout.height as u16).max(1).min(height)
} else {
    // Inline mode: always use full terminal height for fixed-bottom layouts
    height
};
```

---

## 3. Component System

### 3.1 Box Component

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/components/box_component.rs`**

The Box component is a flexbox container with builder pattern:

```rust
// Lines 9-16
pub struct Box {
    style: Style,
    children: Vec<Element>,
    key: Option<String>,
    scroll_offset_x: Option<u16>,
    scroll_offset_y: Option<u16>,
}
```

Usage:
```rust
Box::new()
    .padding(1)
    .flex_direction(FlexDirection::Column)
    .border_style(BorderStyle::Round)
    .child(Text::new("Hello").into_element())
    .into_element()
```

### 3.2 Text Component

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/components/text.rs`**

Text supports both simple and rich text with spans:

```rust
// Lines 196-209
pub struct Text {
    lines: Vec<Line>,
    style: Style,
    key: Option<String>,
}

// Lines 27-33 - Span for rich text
pub struct Span {
    pub content: String,
    pub style: Style,
}

// Lines 130-135 - Line (multiple spans)
pub struct Line {
    pub spans: Vec<Span>,
}
```

Usage:
```rust
// Simple text
Text::new("Hello World").color(Color::Green).bold()

// Rich text with multiple styles
Text::spans(vec![
    Span::new("Hello ").color(Color::White),
    Span::new("World").color(Color::Green).bold(),
])
```

### 3.3 Component Registry

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/components/mod.rs`**

Available components:
- `Box` - Flexbox container
- `Text`, `Span`, `Line` - Text with styling
- `List`, `ListItem` - List rendering
- `Table`, `Row`, `Cell` - Table rendering
- `Progress`, `Gauge` - Progress indicators
- `Spinner` - Loading spinner
- `Scrollbar` - Scroll indicator
- `Tabs`, `Tab` - Tab navigation
- `BarChart`, `Sparkline` - Data visualization
- `Message`, `ToolCall`, `ThinkingBlock` - Chat UI
- `Static` - Static output for persistent content
- `ScrollableBox` - Scrollable container
- `Transform` - Text transformation

---

## 4. State Management

### 4.1 use_signal Hook

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/hooks/use_signal.rs`**

The `Signal` is a reactive state container that triggers re-renders when updated:

```rust
// Lines 8-12
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    render_callback: Option<RenderCallback>,
}

impl<T> Signal<T> {
    // Lines 24-29 - Get value
    pub fn get(&self) -> T where T: Clone {
        self.value.borrow().clone()
    }

    // Lines 37-40 - Set value and trigger re-render
    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.trigger_render();
    }

    // Lines 43-46 - Update with function
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.borrow_mut());
        self.trigger_render();
    }

    // Lines 49-51 - Silent update (no re-render)
    pub fn set_silent(&self, value: T) {
        *self.value.borrow_mut() = value;
    }
}
```

Usage:
```rust
let count = use_signal(|| 0);
count.set(count.get() + 1);
```

### 4.2 use_scroll Hook

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/hooks/use_scroll.rs`**

Manages scroll state for scrollable content:

```rust
// Lines 8-22
pub struct ScrollState {
    pub offset_y: usize,
    pub offset_x: usize,
    pub content_height: usize,
    pub content_width: usize,
    pub viewport_height: usize,
    pub viewport_width: usize,
}

// Lines 186-191
pub struct ScrollHandle {
    state: std::rc::Rc<RefCell<ScrollState>>,
}
```

Methods include:
- `scroll_up()`, `scroll_down()`
- `page_up()`, `page_down()`
- `scroll_to_top()`, `scroll_to_bottom()`
- `scroll_to_item(index)`
- `visible_range()` - returns (start, end) for virtual scrolling

---

## 5. Input Handling

### 5.1 use_input Hook

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/hooks/use_input.rs`**

```rust
// Lines 6-24
pub struct Key {
    pub up_arrow: bool,
    pub down_arrow: bool,
    pub left_arrow: bool,
    pub right_arrow: bool,
    pub page_up: bool,
    pub page_down: bool,
    pub home: bool,
    pub end: bool,
    pub return_key: bool,
    pub escape: bool,
    pub tab: bool,
    pub backspace: bool,
    pub delete: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

// Lines 133-140
pub fn use_input<F>(handler: F)
where
    F: Fn(&str, &Key) + 'static,
{
    register_input_handler(handler);
}
```

Usage:
```rust
use_input(|input, key| {
    if key.up_arrow {
        // Handle up arrow
    }
    if input == "q" {
        // Handle 'q' key
    }
});
```

---

## 6. Terminal Handling

### 6.1 ANSI Escape Codes

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/terminal.rs`, Lines 24-122**

```rust
mod ansi {
    /// Move cursor to specific position (1-indexed)
    pub fn cursor_to(row: u16, col: u16) -> String {
        format!("\x1b[{};{}H", row + 1, col + 1)
    }

    /// Move cursor to home position (0, 0)
    pub fn cursor_home() -> &'static str {
        "\x1b[H"
    }

    /// Move cursor up n lines
    pub fn cursor_up(n: u16) -> String {
        if n == 0 { String::new() } else { format!("\x1b[{}A", n) }
    }

    /// Erase from cursor to end of line
    pub fn erase_end_of_line() -> &'static str {
        "\x1b[K"
    }

    /// Erase entire line
    pub fn erase_line() -> &'static str {
        "\x1b[2K"
    }

    /// Erase entire screen
    pub fn erase_screen() -> &'static str {
        "\x1b[2J"
    }

    /// Hide cursor
    pub fn hide_cursor() -> &'static str {
        "\x1b[?25l"
    }

    /// Show cursor
    pub fn show_cursor() -> &'static str {
        "\x1b[?25h"
    }

    /// Enter alternate screen buffer
    pub fn enter_alt_screen() -> &'static str {
        "\x1b[?1049h"
    }

    /// Leave alternate screen buffer
    pub fn leave_alt_screen() -> &'static str {
        "\x1b[?1049l"
    }
}
```

---

## 7. Cross-Thread Rendering

**File: `/Users/apple/Desktop/code/AI/tool/tink/src/renderer/app.rs`**

For async/multi-threaded apps, rnk provides cross-thread render requests:

```rust
// Lines 27-28
static GLOBAL_RENDER_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

// Lines 102-106
pub fn request_render() {
    if let Some(flag) = GLOBAL_RENDER_FLAG.get() {
        flag.store(true, Ordering::SeqCst);
    }
}

// Lines 265-295 - RenderHandle for thread-safe access
#[derive(Clone)]
pub struct RenderHandle {
    flag: Arc<AtomicBool>,
}

impl RenderHandle {
    pub fn request_render(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
    pub fn println(&self, message: impl IntoPrintable) {
        println(message);
    }
    pub fn enter_alt_screen(&self) {
        enter_alt_screen();
    }
    pub fn exit_alt_screen(&self) {
        exit_alt_screen();
    }
}
```

---

## 8. Key Public APIs

### Entry Points

```rust
// Inline mode (default)
render(my_app).run()?;

// Fullscreen mode
render(my_app).fullscreen().run()?;

// Render element to string (for testing/CLI)
let output = render_to_string(&element, 80);
```

### Hooks Summary

- `use_signal(|| initial_value)` - Reactive state
- `use_input(|input, key| { ... })` - Keyboard input
- `use_scroll()` - Scroll state management
- `use_effect(|| { ... }, deps)` - Side effects
- `use_app()` - App control (exit, mode switch)
- `use_mouse(|action| { ... })` - Mouse input
- `use_focus()` - Focus management
- `use_measure()` - Element measurement

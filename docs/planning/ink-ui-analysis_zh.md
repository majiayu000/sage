# Ink TUI Framework Analysis

## Overview

Ink (version 5.x) is a React-based terminal UI framework for Node.js/JavaScript, providing React's component model and hooks for building CLI applications. It's the framework used by Claude Code (open-claude-code).

---

## 1. Rendering Architecture

### 1.1 React Reconciler

Ink uses a custom React reconciler to render React components to the terminal:

```javascript
// Custom host config for react-reconciler
const reconciler = ReactReconciler({
  createInstance(type, props) {
    return new DOMElement(type);
  },
  appendChild(parent, child) {
    parent.appendChild(child);
  },
  // ... other methods
});
```

### 1.2 Yoga Layout Engine

Ink uses Yoga (Facebook's Flexbox layout engine written in C++) for layout calculations:

```javascript
import Yoga from 'yoga-layout-prebuilt';

// Each element gets a Yoga node
const node = Yoga.Node.create();
node.setFlexDirection(Yoga.FLEX_DIRECTION_ROW);
node.setWidth(100);
```

### 1.3 Rendering Modes

**Inline Mode (Default):**
- Renders at current cursor position
- Content persists in terminal history
- Uses clear-and-rewrite strategy for updates

**Fullscreen Mode:**
- Uses alternate screen buffer
- Content cleared on exit
- Enabled with `fullScreen: true`

```javascript
import {render} from 'ink';

// Inline mode
render(<App />);

// Fullscreen mode
render(<App />, {fullScreen: true});
```

---

## 2. Component System

### 2.1 Built-in Components

**Box** - Flexbox container:
```jsx
<Box flexDirection="column" padding={1} borderStyle="round">
  <Text>Hello World</Text>
</Box>
```

**Text** - Styled text output:
```jsx
<Text color="green" bold>
  Success!
</Text>
```

**Static** - Non-updating content:
```jsx
<Static items={logs}>
  {log => <Text key={log.id}>{log.message}</Text>}
</Static>
```

**Newline**, **Spacer** - Layout utilities

### 2.2 Community Components

- ink-text-input - Text input field
- ink-select-input - Selection menu
- ink-spinner - Loading spinner
- ink-progress-bar - Progress indicator
- ink-table - Table rendering

---

## 3. State Management

### 3.1 React Hooks

Ink uses standard React hooks:

```javascript
import {useState, useEffect} from 'react';

function App() {
  const [count, setCount] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setCount(c => c + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  return <Text>Count: {count}</Text>;
}
```

### 3.2 Custom Hooks

**useInput** - Keyboard input:
```javascript
import {useInput} from 'ink';

function App() {
  useInput((input, key) => {
    if (key.escape) {
      process.exit();
    }
    if (key.upArrow) {
      // Handle up arrow
    }
  });
}
```

**useApp** - App control:
```javascript
import {useApp} from 'ink';

function App() {
  const {exit} = useApp();

  return (
    <Text onSubmit={() => exit()}>
      Press Enter to exit
    </Text>
  );
}
```

**useStdin** - Raw stdin access:
```javascript
import {useStdin} from 'ink';

function App() {
  const {isRawModeSupported, setRawMode} = useStdin();

  useEffect(() => {
    setRawMode(true);
    return () => setRawMode(false);
  }, []);
}
```

**useStdout** - Terminal info:
```javascript
import {useStdout} from 'ink';

function App() {
  const {stdout} = useStdout();
  const width = stdout.columns;
}
```

---

## 4. Layout System

### 4.1 Flexbox Properties

Ink supports full CSS Flexbox:

```jsx
<Box
  flexDirection="row"        // row, column, row-reverse, column-reverse
  flexWrap="wrap"            // wrap, nowrap
  alignItems="center"        // flex-start, flex-end, center, stretch
  justifyContent="space-between"  // flex-start, flex-end, center, space-between, space-around
  flexGrow={1}
  flexShrink={0}
  width="100%"
  height={10}
  padding={1}
  margin={2}
  gap={1}
>
  <Box flexBasis="50%">Left</Box>
  <Box flexBasis="50%">Right</Box>
</Box>
```

### 4.2 Fixed-Bottom Layout Pattern

```jsx
function ChatApp() {
  return (
    <Box flexDirection="column" height="100%">
      {/* Content area takes remaining space */}
      <Box flexGrow={1} flexDirection="column">
        <Static items={messages}>
          {msg => <Message key={msg.id} {...msg} />}
        </Static>
      </Box>

      {/* Fixed bottom */}
      <Box>
        <Text>─────────────────</Text>
      </Box>
      <Box>
        <TextInput value={input} onChange={setInput} />
      </Box>
      <Box>
        <Text dimColor>Status bar</Text>
      </Box>
    </Box>
  );
}
```

---

## 5. Input Handling

### 5.1 Key Object Structure

```typescript
interface Key {
  upArrow: boolean;
  downArrow: boolean;
  leftArrow: boolean;
  rightArrow: boolean;
  pageUp: boolean;
  pageDown: boolean;
  return: boolean;
  escape: boolean;
  ctrl: boolean;
  shift: boolean;
  tab: boolean;
  backspace: boolean;
  delete: boolean;
  meta: boolean;
}
```

### 5.2 Raw Mode

Ink manages raw mode automatically when using `useInput`:

```javascript
// Raw mode enabled during active input
useInput((input, key) => {
  // Handle input
}, {isActive: true});
```

---

## 6. Terminal Handling

### 6.1 Terminal Detection

Ink detects 20+ terminals/IDEs:

```javascript
const terminals = [
  'iTerm.app',
  'Apple_Terminal',
  'vscode',
  'Hyper',
  'Alacritty',
  'Terminus',
  // ... 15+ more
];
```

### 6.2 Flicker Prevention

```javascript
// Cursor hiding during render
stdout.write(ansi.cursorHide);

// Batch writes
const output = [];
for (const line of lines) {
  output.push(line);
}
stdout.write(output.join('\n'));

// Cursor restore after render
stdout.write(ansi.cursorShow);
```

### 6.3 Diff-Based Updates

Ink only updates changed lines:

```javascript
// Compare previous and current output
const changes = diff(previousOutput, currentOutput);

// Only write changed lines
for (const change of changes) {
  moveCursor(change.line);
  clearLine();
  write(change.content);
}
```

---

## 7. Focus Management

### 7.1 useFocus Hook

```javascript
import {useFocus} from 'ink';

function Input({id}) {
  const {isFocused} = useFocus({id});

  return (
    <Box borderStyle={isFocused ? 'bold' : 'single'}>
      <Text>{isFocused ? '>' : ' '} Input</Text>
    </Box>
  );
}
```

### 7.2 useFocusManager

```javascript
import {useFocusManager} from 'ink';

function App() {
  const {focusNext, focusPrevious} = useFocusManager();

  useInput((input, key) => {
    if (key.tab) {
      if (key.shift) {
        focusPrevious();
      } else {
        focusNext();
      }
    }
  });
}
```

---

## 8. Static Component

The `Static` component is crucial for performance in long-running apps:

```jsx
import {Static, Box, Text} from 'ink';

function App() {
  const [logs, setLogs] = useState([]);

  return (
    <Box flexDirection="column">
      {/* Static content - only rendered once per item */}
      <Static items={logs}>
        {log => (
          <Text key={log.id}>
            [{log.timestamp}] {log.message}
          </Text>
        )}
      </Static>

      {/* Dynamic content - re-rendered on every update */}
      <Box>
        <Spinner /> Processing...
      </Box>
    </Box>
  );
}
```

---

## 9. Context Providers

Ink uses multiple context layers:

```jsx
// Internal context structure
<AppProvider>
  <StdoutProvider>
    <StdinProvider>
      <FocusProvider>
        <App />
      </FocusProvider>
    </StdinProvider>
  </StdoutProvider>
</AppProvider>
```

---

## 10. API Summary

### Entry Points

```javascript
// Basic render
const {unmount, rerender, waitUntilExit} = render(<App />);

// With options
render(<App />, {
  stdout: process.stdout,
  stdin: process.stdin,
  exitOnCtrlC: true,
  patchConsole: true,
  debug: false,
});

// Measure text without rendering
const {width, height} = measureElement(<Component />);
```

### Components

- `Box` - Flexbox container
- `Text` - Styled text
- `Static` - Non-updating content
- `Newline` - Line break
- `Spacer` - Flexible space
- `Transform` - Text transformation

### Hooks

- `useInput(handler, options)` - Keyboard input
- `useApp()` - App control (exit)
- `useFocus(options)` - Focus state
- `useFocusManager()` - Focus navigation
- `useStdin()` - Raw stdin access
- `useStdout()` - Terminal info

---

## 11. Claude Code Usage Patterns

From the open-claude-code codebase:

### Component Organization
- 206 React components total
- 15 Ink primitive usage
- Heavy use of custom hooks

### Key Patterns
1. **Context for state** - Multiple context providers for different concerns
2. **Static for history** - Completed messages use Static component
3. **Inline mode** - Default rendering without alternate screen
4. **Component ordering** - Fixed bottom via component order, not CSS position

### Example Structure

```jsx
function ChatApp() {
  return (
    <AppProvider>
      <ThemeProvider>
        <FocusProvider>
          <Box flexDirection="column">
            <MessageHistory />
            <Separator />
            <InputLine />
            <StatusBar />
          </Box>
        </FocusProvider>
      </ThemeProvider>
    </AppProvider>
  );
}
```

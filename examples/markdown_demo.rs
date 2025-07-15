//! Markdown rendering demo

use sage_core::ui::{DisplayManager, render_markdown};

fn main() {
    let markdown_content = r#"
# Sage Agent - Markdown Demo

This is a demonstration of **markdown rendering** in the terminal.

## Features

- **Headers**: Different levels of headers with styling
- **Lists**: Both ordered and unordered lists
- **Code**: Inline `code` and code blocks
- **Emphasis**: *Italic* and **bold** text
- **Links**: [Sage Agent](https://github.com/example/sage-agent)

### Code Example

```rust
fn main() {
    println!("Hello, Sage Agent!");
    let agent = SageAgent::new()?;
    agent.run().await?;
}
```

```python
def create_agent():
    """Create a new Sage agent instance"""
    return SageAgent(
        model="gpt-4",
        temperature=0.7
    )
```

### Lists

1. First item
2. Second item
   - Nested item
   - Another nested item
3. Third item

### Blockquotes

> This is a blockquote example.
> It can span multiple lines.

### More Examples

- Simple list item
- Another item with **bold** text
- Item with `inline code`

That's all for now! 🚀
"#;

    println!("🎨 Markdown Rendering Demo");
    println!("{}", "=".repeat(50));

    // Render the markdown content
    DisplayManager::print_markdown(markdown_content);

    println!("{}", "\n".repeat(2));
    println!("🎯 Raw markdown for comparison:");
    println!("{}", "-".repeat(30));
    println!("{}", markdown_content);
}

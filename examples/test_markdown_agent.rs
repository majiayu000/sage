//! Test markdown rendering in agent responses

use sage_core::{agent::base::BaseAgent, ui::DisplayManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mock LLM response with markdown content
    let markdown_response = r#"# Task Analysis

I'll help you with this task. Here's my analysis:

## Approach

1. **First step**: Analyze the requirements
2. **Second step**: Implement the solution
3. **Third step**: Test and validate

### Code Example

```rust
fn solve_problem() -> Result<(), Error> {
    println!("Solving the problem...");
    Ok(())
}
```

### Key Points

- Use **clean architecture** patterns
- Implement proper *error handling*
- Add comprehensive `unit tests`

> **Note**: This is a complex task that requires careful planning.

That's my recommendation! 🚀"#;

    println!("🧪 Testing Markdown Rendering in Agent Response");
    println!("{}", "=".repeat(60));

    // Test the is_markdown_content detection
    println!("\n📋 Testing Markdown Detection:");
    println!(
        "Is markdown: {}",
        BaseAgent::is_markdown_content(markdown_response)
    );

    // Test direct markdown rendering
    println!("\n🎨 Direct Markdown Rendering:");
    DisplayManager::print_markdown(markdown_response);

    // Test with plain text
    let plain_text = "This is just plain text without any markdown formatting.";
    println!("\n📝 Testing Plain Text:");
    println!(
        "Is markdown: {}",
        BaseAgent::is_markdown_content(plain_text)
    );
    println!("Plain text: {}", plain_text);

    Ok(())
}

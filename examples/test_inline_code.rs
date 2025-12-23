//! Test Inline Code Rendering
//!
//! This example demonstrates the markdown rendering functionality,
//! specifically testing how inline code blocks are rendered in the terminal UI.

use sage_core::ui::render_markdown;

fn main() {
    let markdown = r#"
This is a test of `inline code` functionality.
"#;

    println!("Testing inline code rendering:\n");
    render_markdown(markdown);
}

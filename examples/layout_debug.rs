//! Debug: Simple layout test
//!
//! Run with: cargo run --example layout_debug

use rnk::prelude::*;

fn app() -> Element {
    let app_ctx = use_app();

    use_input({
        let app_ctx = app_ctx.clone();
        move |ch, key| {
            if key.ctrl && ch == "c" {
                app_ctx.exit();
            }
        }
    });

    // Get terminal size
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));

    // Simple column layout with 5 elements
    Box::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(format!("Line 1 - Terminal: {}x{}", term_width, term_height)).into_element(),
        )
        .child(Text::new("Line 2 - Content area").into_element())
        .child(Text::new("Line 3 - Separator").dim().into_element())
        .child(
            Text::new("Line 4 - Input")
                .color(Color::Yellow)
                .into_element(),
        )
        .child(
            Text::new("Line 5 - Status")
                .color(Color::Cyan)
                .into_element(),
        )
        .into_element()
}

fn main() -> std::io::Result<()> {
    render(app).run()
}


use crossterm::{
    style::{Color, Stylize},
    QueueableCommand,
};
use std::io::{self, Write};
use sage_core::ui::markdown::Markdown;

fn main() -> io::Result<()> {
    let markdown = r#"
This is a test of `inline code` functionality.
"#;

    let mut stdout = io::stdout();
    let md = Markdown::from(markdown);
    md.draw(&mut stdout)?;
    stdout.flush()?;

    Ok(())
}

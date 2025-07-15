//! Markdown rendering for terminal output

use colored::*;
use pulldown_cmark::{Event, Parser, Tag};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use textwrap::{wrap, Options};

/// Markdown renderer for terminal output
pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    width: usize,
}

impl MarkdownRenderer {
    /// Create a new markdown renderer
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            width: 80, // Default terminal width
        }
    }

    /// Set the terminal width for text wrapping
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Render markdown text to colored terminal output
    pub fn render(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut output = String::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content = String::new();
        let mut list_depth: usize = 0;
        let mut in_heading = false;
        let mut heading_level: usize = 0;
        let mut in_emphasis = false;
        let mut in_strong = false;
        let mut in_paragraph = false;

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading(level, _, _) => {
                            in_heading = true;
                            heading_level = level as usize;
                            output.push('\n');
                        }
                        Tag::Paragraph => {
                            in_paragraph = true;
                            if !output.is_empty() && !output.ends_with('\n') {
                                output.push('\n');
                            }
                        }
                        Tag::List(_) => {
                            list_depth += 1;
                            output.push('\n');
                        }
                        Tag::Item => {
                            let indent = "  ".repeat(list_depth.saturating_sub(1));
                            output.push_str(&format!("{}• ", indent));
                        }
                        Tag::CodeBlock(kind) => {
                            in_code_block = true;
                            if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                                code_lang = lang.to_string();
                            }
                            output.push('\n');
                        }
                        Tag::Emphasis => {
                            in_emphasis = true;
                        }
                        Tag::Strong => {
                            in_strong = true;
                        }
                        Tag::Link(_, dest_url, _) => {
                            output.push_str(&format!("{}", dest_url.blue().underline()));
                        }
                        Tag::BlockQuote => {
                            output.push_str(&"│ ".bright_black());
                        }
                        _ => {}
                    }
                }
                Event::End(tag) => {
                    match tag {
                        Tag::Heading(_, _, _) => {
                            in_heading = false;
                            output.push('\n');
                        }
                        Tag::Paragraph => {
                            in_paragraph = false;
                            output.push('\n');
                        }
                        Tag::List(_) => {
                            list_depth = list_depth.saturating_sub(1);
                            if list_depth == 0 {
                                output.push('\n');
                            }
                        }
                        Tag::Item => {
                            output.push('\n');
                        }
                        Tag::CodeBlock(_) => {
                            if in_code_block {
                                let highlighted = self.highlight_code(&code_content, &code_lang);
                                output.push_str(&highlighted);
                                output.push('\n');
                                in_code_block = false;
                                code_content.clear();
                                code_lang.clear();
                            }
                        }
                        Tag::Emphasis => {
                            in_emphasis = false;
                        }
                        Tag::Strong => {
                            in_strong = false;
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_content.push_str(&text);
                    } else {
                        let formatted_text = if in_heading {
                            self.format_heading(&text, heading_level)
                        } else if in_emphasis {
                            text.italic().to_string()
                        } else if in_strong {
                            text.bold().to_string()
                        } else {
                            self.wrap_text(&text)
                        };
                        output.push_str(&formatted_text);
                    }
                }
                Event::Code(code) => {
                    // Inline code
                    output.push_str(&self.format_inline_code(&code));
                }
                Event::SoftBreak => {
                    if !in_code_block {
                        output.push(' ');
                    }
                }
                Event::HardBreak => {
                    output.push('\n');
                }
                _ => {}
            }
        }

        output
    }

    /// Format inline code with a background color
    fn format_inline_code(&self, code: &str) -> String {
        format!(" {} ", code.black().on_truecolor(240, 240, 240))
    }

    /// Format heading text with appropriate styling
    fn format_heading(&self, text: &str, level: usize) -> String {
        match level {
            1 => format!("{}\n{}", text.bright_blue().bold(), "=".repeat(text.len()).bright_blue()),
            2 => format!("{}\n{}", text.bright_green().bold(), "-".repeat(text.len()).bright_green()),
            3 => format!("{}", text.bright_yellow().bold()),
            4 => format!("{}", text.bright_magenta().bold()),
            5 => format!("{}", text.bright_cyan().bold()),
            _ => format!("{}", text.bold()),
        }
    }

    /// Wrap text to terminal width
    fn wrap_text(&self, text: &str) -> String {
        let options = Options::new(self.width)
            .initial_indent("")
            .subsequent_indent("");
        
        wrap(text, &options).join("\n")
    }

    /// Highlight code with syntax highlighting
    fn highlight_code(&self, code: &str, lang: &str) -> String {
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        
        let syntax = self.syntax_set
            .find_syntax_by_extension(lang)
            .or_else(|| self.syntax_set.find_syntax_by_name(lang))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut output = String::new();

        // Add code block border
        output.push_str(&"┌".bright_black().to_string());
        output.push_str(&"─".repeat(self.width.saturating_sub(2)).bright_black().to_string());
        output.push_str(&"┐\n".bright_black().to_string());

        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            
            output.push_str(&"│ ".bright_black().to_string());
            let highlighted = as_24_bit_terminal_escaped(&ranges[..], false);
            output.push_str(&highlighted);
            
            if !line.ends_with('\n') {
                output.push('\n');
            }
        }

        // Add bottom border
        output.push_str(&"└".bright_black().to_string());
        output.push_str(&"─".repeat(self.width.saturating_sub(2)).bright_black().to_string());
        output.push_str(&"┘".bright_black().to_string());

        output
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Render markdown text to terminal output
pub fn render_markdown(text: &str) -> String {
    MarkdownRenderer::new().render(text)
}

/// Render markdown text with custom width
pub fn render_markdown_with_width(text: &str, width: usize) -> String {
    MarkdownRenderer::new().with_width(width).render(text)
}

//! Template parser
//!
//! Parses template strings into AST nodes.

use super::types::*;

/// Template parser
pub struct TemplateParser;

impl TemplateParser {
    /// Parse a template string into a Template AST
    pub fn parse(input: &str) -> Template {
        let nodes = Self::parse_nodes(input);
        Template::new(nodes)
    }

    /// Parse template string into nodes
    pub(super) fn parse_nodes(input: &str) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut pos = 0;
        let mut text_start = 0;

        while pos < chars.len() {
            if pos + 1 < chars.len() && chars[pos] == '$' && chars[pos + 1] == '{' {
                if pos > text_start {
                    let text: String = chars[text_start..pos].iter().collect();
                    if !text.is_empty() {
                        nodes.push(TemplateNode::Text(text));
                    }
                }

                let expr_start = pos + 2;
                if let Some(expr_end) = Self::find_matching_brace(&chars, expr_start) {
                    let expr: String = chars[expr_start..expr_end].iter().collect();
                    if let Some(node) = super::expressions::parse_expression(&expr) {
                        nodes.push(node);
                    }
                    pos = expr_end + 1;
                    text_start = pos;
                } else {
                    pos += 1;
                }
            } else {
                pos += 1;
            }
        }

        if text_start < chars.len() {
            let text: String = chars[text_start..].iter().collect();
            if !text.is_empty() {
                nodes.push(TemplateNode::Text(text));
            }
        }

        nodes
    }

    /// Find the matching closing brace, handling nested braces and backticks
    fn find_matching_brace(chars: &[char], start: usize) -> Option<usize> {
        let mut depth = 1;
        let mut in_backtick = false;
        let mut pos = start;

        while pos < chars.len() {
            let c = chars[pos];

            if c == '`' {
                in_backtick = !in_backtick;
            } else if !in_backtick {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        return Some(pos);
                    }
                }
            }
            pos += 1;
        }

        None
    }

    /// Parse a lambda expression (public for tests)
    pub fn parse_lambda(expr: &str) -> Option<LambdaExpr> {
        super::arguments::parse_lambda(expr)
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;

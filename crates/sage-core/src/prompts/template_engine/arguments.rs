//! Argument parsing for template engine

use super::expressions::parse_variable_ref;
use super::types::*;

/// Parse function arguments
pub(super) fn parse_function_args(args_str: &str) -> Vec<FunctionArg> {
    if args_str.trim().is_empty() {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = '"';
    let mut depth = 0;

    for c in args_str.chars() {
        match c {
            '"' | '\'' if !in_string => {
                in_string = true;
                string_char = c;
                current.push(c);
            }
            c if c == string_char && in_string => {
                in_string = false;
                current.push(c);
            }
            '(' | '[' | '{' if !in_string => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' if !in_string => {
                depth -= 1;
                current.push(c);
            }
            ',' if !in_string && depth == 0 => {
                if !current.trim().is_empty() {
                    args.push(parse_single_arg(current.trim()));
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        args.push(parse_single_arg(current.trim()));
    }

    args
}

/// Parse a single function argument
fn parse_single_arg(arg: &str) -> FunctionArg {
    let arg = arg.trim();

    // String literal
    if (arg.starts_with('"') && arg.ends_with('"'))
        || (arg.starts_with('\'') && arg.ends_with('\''))
    {
        return FunctionArg::String(arg[1..arg.len() - 1].to_string());
    }

    // Boolean literal
    if arg == "true" {
        return FunctionArg::Bool(true);
    }
    if arg == "false" {
        return FunctionArg::Bool(false);
    }

    // Number literal
    if let Ok(n) = arg.parse::<f64>() {
        return FunctionArg::Number(n);
    }

    // Lambda expression: x => x.name
    if arg.contains("=>") {
        if let Some(lambda) = parse_lambda(arg) {
            return FunctionArg::Lambda(lambda);
        }
    }

    // Variable reference
    FunctionArg::Variable(parse_variable_ref(arg))
}

/// Parse a lambda expression
pub(super) fn parse_lambda(expr: &str) -> Option<LambdaExpr> {
    let parts: Vec<&str> = expr.splitn(2, "=>").collect();
    if parts.len() != 2 {
        return None;
    }

    let param = parts[0].trim().to_string();
    let body_str = parts[1].trim();

    let body = Box::new(TemplateNode::Variable(parse_variable_ref(body_str)));

    Some(LambdaExpr { param, body })
}

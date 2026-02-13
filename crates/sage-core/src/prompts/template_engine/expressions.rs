//! Expression parsing for template engine

use super::types::*;

/// Parse an expression inside ${}
pub(super) fn parse_expression(expr: &str) -> Option<TemplateNode> {
    let expr = expr.trim();

    // Check for conditional: COND?`true`:`false`
    if let Some(cond_pos) = expr.find('?') {
        if expr[cond_pos..].contains('`') {
            return parse_conditional(expr);
        }
    }

    // Check for function call: fn(args)
    if expr.contains('(') && expr.contains(')') {
        return parse_function_call(expr);
    }

    // Simple variable or property access
    Some(TemplateNode::Variable(parse_variable_ref(expr)))
}

/// Parse a variable reference (possibly with property path)
pub(super) fn parse_variable_ref(expr: &str) -> VariableRef {
    let parts: Vec<&str> = expr.split('.').collect();
    if parts.len() == 1 {
        VariableRef::new(parts[0].trim())
    } else {
        let name = parts[0].trim().to_string();
        let path: Vec<String> = parts[1..].iter().map(|s| s.trim().to_string()).collect();
        VariableRef::with_path(name, path)
    }
}

/// Parse a conditional expression
fn parse_conditional(expr: &str) -> Option<TemplateNode> {
    let cond_end = expr.find('?')?;
    let condition_str = expr[..cond_end].trim();
    let condition = ConditionType::parse(condition_str);

    let rest = &expr[cond_end + 1..];

    let true_start = rest.find('`')? + 1;
    let true_end = find_matching_backtick(&rest[true_start..])?;
    let true_content = &rest[true_start..true_start + true_end];
    let after_true = true_start + true_end + 1;
    let false_content = if after_true < rest.len() {
        let remaining = &rest[after_true..];
        if let Some(colon_pos) = remaining.find(':') {
            let after_colon = &remaining[colon_pos + 1..];
            if let Some(false_start) = after_colon.find('`') {
                let false_inner = &after_colon[false_start + 1..];
                if let Some(false_end) = find_matching_backtick(false_inner) {
                    &false_inner[..false_end]
                } else {
                    ""
                }
            } else {
                ""
            }
        } else {
            ""
        }
    } else {
        ""
    };

    let true_branch = super::parser::TemplateParser::parse_nodes(true_content);
    let false_branch = super::parser::TemplateParser::parse_nodes(false_content);

    Some(TemplateNode::Conditional(ConditionalExpr {
        condition,
        true_branch,
        false_branch,
    }))
}

/// Find the position of the matching backtick (handling nested ${})
fn find_matching_backtick(s: &str) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;
    let mut brace_depth = 0;

    while pos < chars.len() {
        let c = chars[pos];

        if c == '$' && pos + 1 < chars.len() && chars[pos + 1] == '{' {
            brace_depth += 1;
            pos += 2;
            continue;
        }

        if c == '}' && brace_depth > 0 {
            brace_depth -= 1;
            pos += 1;
            continue;
        }

        if c == '`' && brace_depth == 0 {
            return Some(pos);
        }

        pos += 1;
    }

    None
}

/// Parse a function call expression
fn parse_function_call(expr: &str) -> Option<TemplateNode> {
    let paren_start = expr.find('(')?;
    let name = expr[..paren_start].trim().to_string();

    let mut depth = 0;
    let mut paren_end = None;
    for (i, c) in expr[paren_start..].chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    paren_end = Some(paren_start + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let paren_end = paren_end?;
    let args_str = &expr[paren_start + 1..paren_end];
    let args = super::arguments::parse_function_args(args_str);

    let chain = if paren_end + 1 < expr.len() {
        let rest = &expr[paren_end + 1..];
        if rest.starts_with('.') {
            parse_method_chain(&rest[1..])
        } else {
            None
        }
    } else {
        None
    };

    Some(TemplateNode::FunctionCall(FunctionCall { name, args, chain }))
}

/// Parse a method chain
pub(super) fn parse_method_chain(expr: &str) -> Option<Box<MethodChain>> {
    if expr.is_empty() {
        return None;
    }

    let paren_start = expr.find('(')?;
    let method = expr[..paren_start].trim().to_string();

    let mut depth = 0;
    let mut paren_end = None;
    for (i, c) in expr[paren_start..].chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    paren_end = Some(paren_start + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let paren_end = paren_end?;
    let args_str = &expr[paren_start + 1..paren_end];
    let args = super::arguments::parse_function_args(args_str);

    let next = if paren_end + 1 < expr.len() {
        let rest = &expr[paren_end + 1..];
        if rest.starts_with('.') {
            parse_method_chain(&rest[1..])
        } else {
            None
        }
    } else {
        None
    };

    Some(Box::new(MethodChain { method, args, next }))
}

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
    fn parse_nodes(input: &str) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut pos = 0;
        let mut text_start = 0;

        while pos < chars.len() {
            // Look for ${
            if pos + 1 < chars.len() && chars[pos] == '$' && chars[pos + 1] == '{' {
                // Add text before this
                if pos > text_start {
                    let text: String = chars[text_start..pos].iter().collect();
                    if !text.is_empty() {
                        nodes.push(TemplateNode::Text(text));
                    }
                }

                // Find matching }
                let expr_start = pos + 2;
                if let Some(expr_end) = Self::find_matching_brace(&chars, expr_start) {
                    let expr: String = chars[expr_start..expr_end].iter().collect();
                    if let Some(node) = Self::parse_expression(&expr) {
                        nodes.push(node);
                    }
                    pos = expr_end + 1;
                    text_start = pos;
                } else {
                    // No matching brace, treat as text
                    pos += 1;
                }
            } else {
                pos += 1;
            }
        }

        // Add remaining text
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

    /// Parse an expression inside ${}
    fn parse_expression(expr: &str) -> Option<TemplateNode> {
        let expr = expr.trim();

        // Check for conditional: COND?`true`:`false`
        if let Some(cond_pos) = expr.find('?') {
            if expr[cond_pos..].contains('`') {
                return Self::parse_conditional(expr);
            }
        }

        // Check for function call: fn(args)
        if expr.contains('(') && expr.contains(')') {
            return Self::parse_function_call(expr);
        }

        // Simple variable or property access
        Some(TemplateNode::Variable(Self::parse_variable_ref(expr)))
    }

    /// Parse a variable reference (possibly with property path)
    fn parse_variable_ref(expr: &str) -> VariableRef {
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
        // Find the condition part (before ?)
        let cond_end = expr.find('?')?;
        let condition_str = expr[..cond_end].trim();
        let condition = ConditionType::parse(condition_str);

        // Parse the branches
        let rest = &expr[cond_end + 1..];

        // Find true branch: `content`
        let true_start = rest.find('`')? + 1;
        let true_end = Self::find_matching_backtick(&rest[true_start..])?;
        let true_content = &rest[true_start..true_start + true_end];

        // Find false branch after `:`
        let after_true = true_start + true_end + 1;
        let false_content = if after_true < rest.len() {
            let remaining = &rest[after_true..];
            if let Some(colon_pos) = remaining.find(':') {
                let after_colon = &remaining[colon_pos + 1..];
                if let Some(false_start) = after_colon.find('`') {
                    let false_inner = &after_colon[false_start + 1..];
                    if let Some(false_end) = Self::find_matching_backtick(false_inner) {
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

        // Parse nested content in branches
        let true_branch = Self::parse_nodes(true_content);
        let false_branch = Self::parse_nodes(false_content);

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

        // Find matching closing paren
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
        let args = Self::parse_function_args(args_str);

        // Check for method chain after the function call
        let chain = if paren_end + 1 < expr.len() {
            let rest = &expr[paren_end + 1..];
            if rest.starts_with('.') {
                Self::parse_method_chain(&rest[1..])
            } else {
                None
            }
        } else {
            None
        };

        Some(TemplateNode::FunctionCall(FunctionCall { name, args, chain }))
    }

    /// Parse function arguments
    fn parse_function_args(args_str: &str) -> Vec<FunctionArg> {
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
                        args.push(Self::parse_single_arg(current.trim()));
                    }
                    current.clear();
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            args.push(Self::parse_single_arg(current.trim()));
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
            if let Some(lambda) = Self::parse_lambda(arg) {
                return FunctionArg::Lambda(lambda);
            }
        }

        // Variable reference
        FunctionArg::Variable(Self::parse_variable_ref(arg))
    }

    /// Parse a lambda expression
    pub fn parse_lambda(expr: &str) -> Option<LambdaExpr> {
        let parts: Vec<&str> = expr.splitn(2, "=>").collect();
        if parts.len() != 2 {
            return None;
        }

        let param = parts[0].trim().to_string();
        let body_str = parts[1].trim();

        // Parse body as a variable reference
        let body = Box::new(TemplateNode::Variable(Self::parse_variable_ref(body_str)));

        Some(LambdaExpr { param, body })
    }

    /// Parse a method chain
    fn parse_method_chain(expr: &str) -> Option<Box<MethodChain>> {
        if expr.is_empty() {
            return None;
        }

        // Find method name and args
        let paren_start = expr.find('(')?;
        let method = expr[..paren_start].trim().to_string();

        // Find matching closing paren
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
        let args = Self::parse_function_args(args_str);

        // Check for next method in chain
        let next = if paren_end + 1 < expr.len() {
            let rest = &expr[paren_end + 1..];
            if rest.starts_with('.') {
                Self::parse_method_chain(&rest[1..])
            } else {
                None
            }
        } else {
            None
        };

        Some(Box::new(MethodChain { method, args, next }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let template = TemplateParser::parse("Hello, world!");
        assert_eq!(template.nodes.len(), 1);
        assert_eq!(
            template.nodes[0],
            TemplateNode::Text("Hello, world!".to_string())
        );
    }

    #[test]
    fn test_parse_simple_variable() {
        let template = TemplateParser::parse("Hello, ${AGENT_NAME}!");
        assert_eq!(template.nodes.len(), 3);
        assert_eq!(template.nodes[0], TemplateNode::Text("Hello, ".to_string()));
        assert_eq!(
            template.nodes[1],
            TemplateNode::Variable(VariableRef::new("AGENT_NAME"))
        );
        assert_eq!(template.nodes[2], TemplateNode::Text("!".to_string()));
    }

    #[test]
    fn test_parse_property_access() {
        let template = TemplateParser::parse("${config.model.name}");
        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            TemplateNode::Variable(var) => {
                assert_eq!(var.name, "config");
                assert_eq!(var.path, vec!["model", "name"]);
            }
            _ => panic!("Expected variable node"),
        }
    }

    #[test]
    fn test_parse_conditional() {
        let template = TemplateParser::parse("${IS_GIT_REPO?`Yes`:`No`}");
        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            TemplateNode::Conditional(cond) => {
                assert_eq!(
                    cond.condition,
                    ConditionType::BoolVar("IS_GIT_REPO".to_string())
                );
                assert_eq!(cond.true_branch.len(), 1);
                assert_eq!(cond.false_branch.len(), 1);
            }
            _ => panic!("Expected conditional node"),
        }
    }

    #[test]
    fn test_parse_has_tool_conditional() {
        let template = TemplateParser::parse("${HAS_TOOL_BASH?`bash available`:`no bash`}");
        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            TemplateNode::Conditional(cond) => {
                assert_eq!(cond.condition, ConditionType::HasTool("BASH".to_string()));
            }
            _ => panic!("Expected conditional node"),
        }
    }

    #[test]
    fn test_parse_multiple_variables() {
        let template =
            TemplateParser::parse("Use ${READ_TOOL_NAME} to read and ${EDIT_TOOL_NAME} to edit.");
        assert_eq!(template.nodes.len(), 5);
    }

    #[test]
    fn test_parse_function_call() {
        let template = TemplateParser::parse("${date.format('YYYY-MM-DD')}");
        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            TemplateNode::FunctionCall(func) => {
                assert_eq!(func.name, "date.format");
                assert_eq!(func.args.len(), 1);
                assert_eq!(func.args[0], FunctionArg::String("YYYY-MM-DD".to_string()));
            }
            _ => panic!("Expected function call node"),
        }
    }

    #[test]
    fn test_parse_lambda() {
        let lambda = TemplateParser::parse_lambda("t => t.name");
        assert!(lambda.is_some());
        let lambda = lambda.unwrap();
        assert_eq!(lambda.param, "t");
    }

    #[test]
    fn test_parse_nested_variable_in_conditional() {
        let template = TemplateParser::parse("${IS_GIT_REPO?`Branch: ${GIT_BRANCH}`:`No git`}");
        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            TemplateNode::Conditional(cond) => {
                assert_eq!(cond.true_branch.len(), 2);
                // First should be text "Branch: "
                assert_eq!(
                    cond.true_branch[0],
                    TemplateNode::Text("Branch: ".to_string())
                );
                // Second should be variable GIT_BRANCH
                match &cond.true_branch[1] {
                    TemplateNode::Variable(var) => {
                        assert_eq!(var.name, "GIT_BRANCH");
                    }
                    _ => panic!("Expected variable node"),
                }
            }
            _ => panic!("Expected conditional node"),
        }
    }
}

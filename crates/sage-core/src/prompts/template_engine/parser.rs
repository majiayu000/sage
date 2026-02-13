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
                assert_eq!(
                    cond.true_branch[0],
                    TemplateNode::Text("Branch: ".to_string())
                );
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

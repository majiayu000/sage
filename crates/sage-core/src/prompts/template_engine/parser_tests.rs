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

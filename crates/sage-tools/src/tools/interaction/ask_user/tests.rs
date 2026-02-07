//! Tests for the AskUserQuestion tool

use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use serde_json::json;
use std::collections::HashMap;

use super::tool::AskUserQuestionTool;

fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
    let arguments = if let serde_json::Value::Object(map) = args {
        map.into_iter().collect()
    } else {
        HashMap::new()
    };

    ToolCall {
        id: id.to_string(),
        name: name.to_string(),
        arguments,
        call_id: None,
    }
}

#[tokio::test]
async fn test_single_question() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-1",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "Which authentication method should we use?",
                "header": "Auth method",
                "options": [
                    {
                        "label": "OAuth 2.0",
                        "description": "Industry standard OAuth 2.0 authentication"
                    },
                    {
                        "label": "JWT",
                        "description": "JSON Web Tokens for stateless auth"
                    }
                ],
                "multi_select": false
            }]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("User Input Required"));
    assert!(output.contains("Auth method"));
    assert!(output.contains("OAuth 2.0"));
    assert!(output.contains("JWT"));
}

#[tokio::test]
async fn test_multiple_questions() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-2",
        "ask_user_question",
        json!({
            "questions": [
                {
                    "question": "Which framework should we use?",
                    "header": "Framework",
                    "options": [
                        {
                            "label": "React",
                            "description": "Popular component-based library"
                        },
                        {
                            "label": "Vue",
                            "description": "Progressive framework"
                        }
                    ]
                },
                {
                    "question": "Which state management library?",
                    "header": "State mgmt",
                    "options": [
                        {
                            "label": "Redux",
                            "description": "Predictable state container"
                        },
                        {
                            "label": "MobX",
                            "description": "Simple, scalable state management"
                        }
                    ]
                }
            ]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("Question 1 [Framework]"));
    assert!(output.contains("Question 2 [State mgmt]"));
}

#[tokio::test]
async fn test_multi_select_question() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-3",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "Which features should we implement?",
                "header": "Features",
                "options": [
                    {
                        "label": "Dark mode",
                        "description": "Support for dark theme"
                    },
                    {
                        "label": "i18n",
                        "description": "Internationalization support"
                    },
                    {
                        "label": "Analytics",
                        "description": "Usage analytics tracking"
                    }
                ],
                "multi_select": true
            }]
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("Multiple selections allowed"));
}

#[tokio::test]
async fn test_header_too_long() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-4",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "Test question",
                "header": "This header is way too long",
                "options": [
                    {
                        "label": "Option 1",
                        "description": "First option"
                    },
                    {
                        "label": "Option 2",
                        "description": "Second option"
                    }
                ]
            }]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("exceeds 12 characters"));
}

#[tokio::test]
async fn test_too_few_options() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-5",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "Test question",
                "header": "Test",
                "options": [
                    {
                        "label": "Only option",
                        "description": "The only choice"
                    }
                ]
            }]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("at least 2 options"));
}

#[tokio::test]
async fn test_too_many_options() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-6",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "Test question",
                "header": "Test",
                "options": [
                    {"label": "Opt 1", "description": "First"},
                    {"label": "Opt 2", "description": "Second"},
                    {"label": "Opt 3", "description": "Third"},
                    {"label": "Opt 4", "description": "Fourth"},
                    {"label": "Opt 5", "description": "Fifth"}
                ]
            }]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("too many options"));
}

#[tokio::test]
async fn test_too_many_questions() {
    let tool = AskUserQuestionTool::new();
    let questions = vec![
        json!({
            "question": "Question 1",
            "header": "Q1",
            "options": [
                {"label": "A", "description": "Option A"},
                {"label": "B", "description": "Option B"}
            ]
        }),
        json!({
            "question": "Question 2",
            "header": "Q2",
            "options": [
                {"label": "A", "description": "Option A"},
                {"label": "B", "description": "Option B"}
            ]
        }),
        json!({
            "question": "Question 3",
            "header": "Q3",
            "options": [
                {"label": "A", "description": "Option A"},
                {"label": "B", "description": "Option B"}
            ]
        }),
        json!({
            "question": "Question 4",
            "header": "Q4",
            "options": [
                {"label": "A", "description": "Option A"},
                {"label": "B", "description": "Option B"}
            ]
        }),
        json!({
            "question": "Question 5",
            "header": "Q5",
            "options": [
                {"label": "A", "description": "Option A"},
                {"label": "B", "description": "Option B"}
            ]
        }),
    ];

    let call = create_tool_call(
        "test-7",
        "ask_user_question",
        json!({
            "questions": questions
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Maximum of 4 questions"));
}

#[tokio::test]
async fn test_empty_question_text() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-8",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "   ",
                "header": "Test",
                "options": [
                    {"label": "A", "description": "Option A"},
                    {"label": "B", "description": "Option B"}
                ]
            }]
        }),
    );

    let result = tool.execute(&call).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("empty question text"));
}

#[tokio::test]
async fn test_with_answers() {
    let tool = AskUserQuestionTool::new();
    let call = create_tool_call(
        "test-9",
        "ask_user_question",
        json!({
            "questions": [{
                "question": "Which framework?",
                "header": "Framework",
                "options": [
                    {"label": "React", "description": "React library"},
                    {"label": "Vue", "description": "Vue framework"}
                ]
            }],
            "answers": {
                "question_1": "React"
            }
        }),
    );

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("User Responses"));
    assert!(output.contains("React"));
}

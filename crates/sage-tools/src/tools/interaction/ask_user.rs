//! AskUserQuestion tool for interactive user input during agent execution

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Represents a single option in a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display text for the option
    pub label: String,
    /// Explanation of what this option means
    pub description: String,
}

/// Represents a single question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// The question text to ask
    pub question: String,
    /// Short label for the question (max 12 chars) like "Auth method", "Library"
    pub header: String,
    /// List of options to choose from (2-4 options)
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    #[serde(default)]
    pub multi_select: bool,
}

/// Tool for asking the user questions during execution
///
/// This tool allows the agent to interactively gather information from the user
/// when it needs clarification or choices to be made. It supports single and
/// multiple choice questions with clear descriptions for each option.
pub struct AskUserQuestionTool;

impl Default for AskUserQuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl AskUserQuestionTool {
    pub fn new() -> Self {
        Self
    }

    /// Validate that questions are well-formed
    fn validate_questions(&self, questions: &[Question]) -> Result<(), ToolError> {
        // Check number of questions
        if questions.is_empty() {
            return Err(ToolError::InvalidArguments(
                "At least one question is required".to_string(),
            ));
        }

        if questions.len() > 4 {
            return Err(ToolError::InvalidArguments(
                "Maximum of 4 questions allowed per call".to_string(),
            ));
        }

        // Validate each question
        for (idx, question) in questions.iter().enumerate() {
            // Check header length
            if question.header.len() > 12 {
                return Err(ToolError::InvalidArguments(format!(
                    "Question {} header '{}' exceeds 12 characters (length: {})",
                    idx + 1,
                    question.header,
                    question.header.len()
                )));
            }

            // Check question text is not empty
            if question.question.trim().is_empty() {
                return Err(ToolError::InvalidArguments(format!(
                    "Question {} has empty question text",
                    idx + 1
                )));
            }

            // Check number of options
            if question.options.len() < 2 {
                return Err(ToolError::InvalidArguments(format!(
                    "Question {} must have at least 2 options",
                    idx + 1
                )));
            }

            if question.options.len() > 4 {
                return Err(ToolError::InvalidArguments(format!(
                    "Question {} has too many options (max 4)",
                    idx + 1
                )));
            }

            // Validate each option
            for (opt_idx, option) in question.options.iter().enumerate() {
                if option.label.trim().is_empty() {
                    return Err(ToolError::InvalidArguments(format!(
                        "Question {} option {} has empty label",
                        idx + 1,
                        opt_idx + 1
                    )));
                }

                if option.description.trim().is_empty() {
                    return Err(ToolError::InvalidArguments(format!(
                        "Question {} option {} has empty description",
                        idx + 1,
                        opt_idx + 1
                    )));
                }
            }
        }

        Ok(())
    }

    /// Format questions for display to the user
    fn format_questions(&self, questions: &[Question]) -> String {
        let mut output = String::from("# User Input Required\n\n");
        output.push_str("The agent needs your input to proceed:\n\n");

        for (idx, question) in questions.iter().enumerate() {
            output.push_str(&format!(
                "## Question {} [{}]\n\n",
                idx + 1,
                question.header
            ));
            output.push_str(&format!("{}\n\n", question.question));

            output.push_str("Options:\n");
            for (opt_idx, option) in question.options.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **{}**: {}\n",
                    opt_idx + 1,
                    option.label,
                    option.description
                ));
            }

            if question.multi_select {
                output.push_str("\n*Multiple selections allowed*\n");
            }

            output.push('\n');
        }

        output.push_str("---\n\n");
        output.push_str("Please respond with your selections. For example:\n");
        output.push_str("- Single question: `1` or `2`\n");
        output.push_str("- Multiple questions: `1, 2, 3`\n");
        output.push_str("- Multi-select: `1,3` for options 1 and 3\n");

        output
    }
}

#[async_trait]
impl Tool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "ask_user_question"
    }

    fn description(&self) -> &str {
        "Ask the user one or more questions during execution to gather information, clarify requirements, or get decisions. \
        Supports single and multiple choice questions with 2-4 options each. Use this when you need user input to proceed \
        with a task or when there are multiple valid approaches and the user should choose."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "Array of 1-4 questions to ask the user",
                        "minItems": 1,
                        "maxItems": 4,
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": {
                                    "type": "string",
                                    "description": "The question text to ask the user"
                                },
                                "header": {
                                    "type": "string",
                                    "description": "Short label for the question (max 12 chars) like 'Auth method', 'Library', 'Framework'",
                                    "maxLength": 12
                                },
                                "options": {
                                    "type": "array",
                                    "description": "Array of 2-4 options for the user to choose from",
                                    "minItems": 2,
                                    "maxItems": 4,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": {
                                                "type": "string",
                                                "description": "Display text for this option"
                                            },
                                            "description": {
                                                "type": "string",
                                                "description": "Explanation of what this option means or does"
                                            }
                                        },
                                        "required": ["label", "description"]
                                    }
                                },
                                "multi_select": {
                                    "type": "boolean",
                                    "description": "Whether multiple options can be selected. Defaults to false.",
                                    "default": false
                                }
                            },
                            "required": ["question", "header", "options"]
                        }
                    },
                    "answers": {
                        "type": "object",
                        "description": "Optional: Previously collected answers for processing. The agent should not provide this on first call.",
                        "additionalProperties": true
                    }
                },
                "required": ["questions"]
            }),
        }
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Parse questions from arguments
        let questions_value = tool_call.arguments.get("questions").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: questions".to_string())
        })?;

        let questions: Vec<Question> = serde_json::from_value(questions_value.clone())
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid questions format: {}", e)))?;

        // Validate questions
        self.validate_questions(&questions)?;

        // Check if this is a response with answers
        if let Some(answers_value) = tool_call.arguments.get("answers") {
            // Process user's answers
            let mut response = String::from("# User Responses\n\n");

            if let Some(answers_obj) = answers_value.as_object() {
                for (idx, question) in questions.iter().enumerate() {
                    let question_key = format!("question_{}", idx + 1);
                    if let Some(answer) = answers_obj.get(&question_key) {
                        response.push_str(&format!(
                            "**[{}]** {}\n",
                            question.header, question.question
                        ));
                        response.push_str(&format!("Answer: {}\n\n", answer));
                    }
                }
            }

            return Ok(ToolResult::success(&tool_call.id, self.name(), response));
        }

        // First call: format questions for user
        let formatted = self.format_questions(&questions);

        Ok(ToolResult::success(&tool_call.id, self.name(), formatted))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        // Basic validation - detailed validation happens in execute
        if !call.arguments.contains_key("questions") {
            return Err(ToolError::InvalidArguments(
                "Missing required parameter: questions".to_string(),
            ));
        }
        Ok(())
    }

    /// This tool requires user interaction - the execution loop should block
    /// and wait for user input via the InputChannel when this tool is called.
    fn requires_user_interaction(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
}

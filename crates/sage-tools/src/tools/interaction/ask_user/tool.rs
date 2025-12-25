//! AskUserQuestion tool implementation

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};

use super::schema::create_schema;
use super::types::Question;
use super::validation::validate_questions;

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

    /// Format user's answers for processing
    fn format_answers(&self, questions: &[Question], answers_value: &serde_json::Value) -> String {
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

        response
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
        create_schema(self.name(), self.description())
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Parse questions from arguments
        let questions_value = tool_call.arguments.get("questions").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: questions".to_string())
        })?;

        let questions: Vec<Question> = serde_json::from_value(questions_value.clone())
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid questions format: {}", e)))?;

        // Validate questions
        validate_questions(&questions)?;

        // Check if this is a response with answers
        if let Some(answers_value) = tool_call.arguments.get("answers") {
            // Process user's answers
            let response = self.format_answers(&questions, answers_value);
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

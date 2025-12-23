//! User interaction handling for the unified executor

use crate::error::{SageError, SageResult};
use crate::input::{InputRequest, Question, QuestionOption};
use crate::tools::types::ToolResult;
use anyhow::Context;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Handle ask_user_question tool call with blocking input
    ///
    /// This method intercepts ask_user_question tool calls and uses the InputChannel
    /// to actually block and wait for user input, implementing the unified loop pattern.
    #[instrument(skip(self, tool_call), fields(tool_call_id = %tool_call.id))]
    pub(super) async fn handle_ask_user_question(
        &mut self,
        tool_call: &crate::tools::types::ToolCall,
    ) -> SageResult<ToolResult> {
        // Stop animation while waiting for user input
        self.animation_manager.stop_animation().await;

        // Parse questions from the tool call arguments
        let questions_value = tool_call
            .arguments
            .get("questions")
            .ok_or_else(|| SageError::agent("ask_user_question missing 'questions' parameter"))?;

        // Build the input request from the questions
        let raw_questions: Vec<serde_json::Value> = serde_json::from_value(questions_value.clone())
            .map_err(|e| SageError::agent(format!("Invalid questions format: {}", e)))?;

        // Convert to Question structs
        let mut questions: Vec<Question> = Vec::new();
        let mut question_text = String::from("User Input Required:\n\n");

        for q in raw_questions.iter() {
            let question_str = q.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let header = q
                .get("header")
                .and_then(|v| v.as_str())
                .unwrap_or("Question");
            let multi_select = q
                .get("multi_select")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            question_text.push_str(&format!("[{}] {}\n", header, question_str));

            let mut options: Vec<QuestionOption> = Vec::new();
            if let Some(opts) = q.get("options").and_then(|v| v.as_array()) {
                for (opt_idx, opt) in opts.iter().enumerate() {
                    let label = opt.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    let description = opt
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    question_text.push_str(&format!(
                        "  {}. {}: {}\n",
                        opt_idx + 1,
                        label,
                        description
                    ));
                    options.push(QuestionOption::new(label, description));
                }
            }

            let mut question = Question::new(question_str, header, options);
            if multi_select {
                question = question.with_multi_select();
            }
            questions.push(question);
            question_text.push('\n');
        }

        // Create input request with structured questions
        let request = InputRequest::questions(questions);

        // Print the question
        println!("\n{}", question_text);

        // Block and wait for user input via InputChannel
        let response = self
            .request_user_input(request)
            .await
            .context("Failed to request user input for ask_user_question tool")?;

        // Check if user cancelled
        if response.is_cancelled() {
            return Err(SageError::Cancelled);
        }

        // Format the response for the agent
        let result_text = if let Some(answers) = response.get_answers() {
            let answers_str: Vec<String> = answers
                .iter()
                .map(|(q, a)| format!("Q: {} -> A: {}", q, a))
                .collect();
            format!("User answered:\n{}", answers_str.join("\n"))
        } else if let Some(text) = response.get_text() {
            format!("User Response:\n\n{}", text)
        } else {
            "User provided no response".to_string()
        };

        Ok(ToolResult::success(
            &tool_call.id,
            "ask_user_question",
            result_text,
        ))
    }
}

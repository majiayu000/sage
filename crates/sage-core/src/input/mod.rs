//! Input channel system for unified execution loop
//!
//! This module provides an async communication channel between the execution loop
//! and the user interface, enabling blocking user input within the agent loop.
//!
//! # Design
//!
//! Following Claude Code's architecture, the execution loop can await on user input
//! instead of exiting and resuming. This is achieved through a bidirectional channel:
//!
//! - Execution loop sends `InputRequest` when user input is needed
//! - UI layer receives the request, displays it, and sends back `InputResponse`
//! - Execution loop receives response and continues (no exit/resume cycle)
//!
//! # Request Types
//!
//! - `InputRequest::Question` - Structured questions (from AskUserQuestion tool)
//! - `InputRequest::Permission` - Tool permission requests
//! - `InputRequest::FreeText` - Free text input (when model needs user response)
//!
//! # Example
//!
//! ```ignore
//! // Create channel pair
//! let (channel, handle) = InputChannel::new(16);
//!
//! // Spawn UI handler
//! tokio::spawn(async move {
//!     while let Some(request) = handle.request_rx.recv().await {
//!         // Display question to user, get input
//!         let response = get_user_input(&request);
//!         handle.response_tx.send(response).await.ok();
//!     }
//! });
//!
//! // In execution loop
//! let response = channel.request_input(request).await?;
//! ```

// Module declarations
mod auto_response;
mod channel;
mod permission;
mod request;
mod response;
mod types;

// Re-export all public types

// Core types
pub use types::{InputContext, InputOption, Question, QuestionOption};

// Permission types
pub use permission::{
    PermissionBehavior, PermissionResult, PermissionSuggestion, RuleDestination, SuggestionType,
};

// Request types
pub use request::{InputRequest, InputRequestKind, LegacyInputRequest};

// Response types
pub use response::{InputResponse, InputResponseKind, LegacyInputResponse};

// Auto-response types
pub use auto_response::{AutoResponse, AutoResponder};

// Channel types
pub use channel::{InputChannel, InputChannelHandle};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_input_channel_basic() {
        let (mut channel, mut handle) = InputChannel::new(16);

        // Spawn responder
        let responder = tokio::spawn(async move {
            if let Some(request) = handle.request_rx.recv().await {
                let response = InputResponse::text(request.id, "user response");
                handle.response_tx.send(response).await.ok();
            }
        });

        let request = InputRequest::simple("What is your choice?");
        let response = channel.request_input(request).await.unwrap();

        assert_eq!(response.get_text(), Some("user response"));
        assert!(!response.is_cancelled());

        responder.abort();
    }

    #[tokio::test]
    async fn test_non_interactive_channel() {
        let mut channel = InputChannel::non_interactive(AutoResponse::AlwaysAllow);

        let request = InputRequest::permission("bash", "Run command", serde_json::json!({}));
        let response = channel.request_input(request).await.unwrap();

        assert!(response.is_permission_granted());
    }

    #[tokio::test]
    async fn test_non_interactive_deny() {
        let mut channel = InputChannel::non_interactive(AutoResponse::AlwaysDeny);

        let request = InputRequest::simple("Question?");
        let response = channel.request_input(request).await.unwrap();

        assert!(response.is_cancelled());
    }

    #[tokio::test]
    async fn test_fail_on_input_channel() {
        let mut channel = InputChannel::fail_on_input();

        let request = InputRequest::simple("Question?");
        let response = channel.request_input(request).await.unwrap();

        assert!(response.is_cancelled());
    }

    #[tokio::test]
    async fn test_questions_request() {
        let mut channel = InputChannel::non_interactive(AutoResponse::AlwaysAllow);

        let questions = vec![Question::new(
            "Which framework?",
            "Framework",
            vec![
                QuestionOption::new("React", "Popular UI library"),
                QuestionOption::new("Vue", "Progressive framework"),
            ],
        )];

        let request = InputRequest::questions(questions);
        let response = channel.request_input(request).await.unwrap();

        let answers = response.get_answers().unwrap();
        assert!(answers.contains_key("Which framework?"));
    }

    #[test]
    fn test_input_option() {
        let opt = InputOption::new("Label", "Description");
        assert_eq!(opt.get_value(), "Label");

        let opt_with_value = InputOption::new("Label", "Desc").with_value("custom_value");
        assert_eq!(opt_with_value.get_value(), "custom_value");
    }

    #[test]
    fn test_question_validation() {
        let valid_question = Question::new(
            "Which one?",
            "Choice",
            vec![
                QuestionOption::new("A", "Option A"),
                QuestionOption::new("B", "Option B"),
            ],
        );
        assert!(valid_question.validate().is_ok());

        let invalid_header = Question::new(
            "Which one?",
            "This header is way too long",
            vec![
                QuestionOption::new("A", "Option A"),
                QuestionOption::new("B", "Option B"),
            ],
        );
        assert!(invalid_header.validate().is_err());

        let too_few_options = Question::new(
            "Which one?",
            "Choice",
            vec![QuestionOption::new("A", "Option A")],
        );
        assert!(too_few_options.validate().is_err());
    }

    #[test]
    fn test_permission_result() {
        let allow = PermissionResult::Allow;
        assert!(allow.is_allowed());
        assert!(!allow.is_denied());
        assert!(!allow.needs_user_input());

        let deny = PermissionResult::Deny {
            message: "Not allowed".to_string(),
        };
        assert!(!deny.is_allowed());
        assert!(deny.is_denied());

        let ask = PermissionResult::Ask {
            message: "Confirm?".to_string(),
            suggestions: vec![],
        };
        assert!(ask.needs_user_input());
    }
}

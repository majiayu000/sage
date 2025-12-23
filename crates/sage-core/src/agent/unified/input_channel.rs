//! Input channel creation and auto-response conversion

use crate::agent::{ExecutionMode, ExecutionOptions};
use crate::input::{AutoResponse, InputChannel, InputRequest, InputRequestKind, InputResponse};

/// Create input channel based on execution mode
pub(super) fn create_input_channel(options: &ExecutionOptions) -> Option<InputChannel> {
    match &options.mode {
        ExecutionMode::Interactive => None, // Will be set externally
        ExecutionMode::NonInteractive { auto_response } => {
            // Convert from agent::AutoResponse to input::AutoResponse
            let input_auto_response = convert_auto_response(auto_response);
            Some(InputChannel::non_interactive(input_auto_response))
        }
        ExecutionMode::Batch => Some(InputChannel::fail_on_input()),
    }
}

/// Convert agent::AutoResponse to input::AutoResponse
fn convert_auto_response(auto_response: &crate::agent::AutoResponse) -> AutoResponse {
    match auto_response {
        crate::agent::AutoResponse::Fixed(text) => {
            let text = text.clone();
            AutoResponse::Custom(std::sync::Arc::new(move |req: &InputRequest| {
                InputResponse::text(req.id, text.clone())
            }))
        }
        crate::agent::AutoResponse::FirstOption => AutoResponse::AlwaysAllow,
        crate::agent::AutoResponse::LastOption => AutoResponse::AlwaysAllow,
        crate::agent::AutoResponse::Cancel => AutoResponse::AlwaysDeny,
        crate::agent::AutoResponse::ContextBased {
            default_text,
            prefer_first_option,
        } => {
            let text = default_text.clone();
            let prefer_first = *prefer_first_option;
            AutoResponse::Custom(std::sync::Arc::new(move |req: &InputRequest| {
                match &req.kind {
                    InputRequestKind::Questions { questions } if prefer_first => {
                        // Select first option for each question
                        let answers: std::collections::HashMap<String, String> = questions
                            .iter()
                            .map(|q| {
                                let answer = q
                                    .options
                                    .first()
                                    .map(|o| o.label.clone())
                                    .unwrap_or_default();
                                (q.question.clone(), answer)
                            })
                            .collect();
                        InputResponse::question_answers(req.id, answers)
                    }
                    InputRequestKind::Simple {
                        options: Some(_), ..
                    } if prefer_first => InputResponse::selected(req.id, 0, "auto-selected"),
                    _ => InputResponse::text(req.id, text.clone()),
                }
            }))
        }
    }
}

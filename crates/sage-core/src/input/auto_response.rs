//! Auto-response strategies for non-interactive mode

use std::collections::HashMap;
use std::sync::Arc;

use super::request::{InputRequest, InputRequestKind};
use super::response::InputResponse;

/// Auto-responder function type
pub type AutoResponder = Box<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>;

/// Auto-response strategies for non-interactive mode
#[derive(Clone)]
pub enum AutoResponse {
    /// Use default responses (empty answers, deny permissions)
    Default,
    /// Always allow permissions, use first option for questions
    AlwaysAllow,
    /// Always deny/cancel
    AlwaysDeny,
    /// Custom responder function
    Custom(Arc<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>),
}

impl std::fmt::Debug for AutoResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutoResponse::Default => write!(f, "AutoResponse::Default"),
            AutoResponse::AlwaysAllow => write!(f, "AutoResponse::AlwaysAllow"),
            AutoResponse::AlwaysDeny => write!(f, "AutoResponse::AlwaysDeny"),
            AutoResponse::Custom(_) => write!(f, "AutoResponse::Custom(...)"),
        }
    }
}

impl AutoResponse {
    /// Convert to a responder function
    pub fn into_responder(self) -> AutoResponder {
        match self {
            AutoResponse::Default => Box::new(|req: &InputRequest| match &req.kind {
                InputRequestKind::Questions { .. } => {
                    InputResponse::question_answers(req.id, HashMap::new())
                }
                InputRequestKind::Permission { .. } => InputResponse::permission_denied(
                    req.id,
                    Some("Non-interactive mode".to_string()),
                ),
                InputRequestKind::FreeText { .. } => InputResponse::free_text(req.id, ""),
                InputRequestKind::Simple { options, .. } => {
                    if options.is_some() {
                        InputResponse::selected(req.id, 0, "")
                    } else {
                        InputResponse::text(req.id, "")
                    }
                }
            }),
            AutoResponse::AlwaysAllow => Box::new(|req: &InputRequest| match &req.kind {
                InputRequestKind::Questions { questions } => {
                    // Select first option for each question
                    let answers: HashMap<String, String> = questions
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
                InputRequestKind::Permission { input, .. } => {
                    InputResponse::permission_granted_with_input(req.id, input.clone())
                }
                InputRequestKind::FreeText { .. } => InputResponse::free_text(req.id, "continue"),
                InputRequestKind::Simple { options, .. } => {
                    if options.is_some() {
                        InputResponse::selected(req.id, 0, "auto-selected")
                    } else {
                        InputResponse::text(req.id, "continue")
                    }
                }
            }),
            AutoResponse::AlwaysDeny => {
                Box::new(|req: &InputRequest| InputResponse::cancelled(req.id))
            }
            AutoResponse::Custom(f) => Box::new(move |req: &InputRequest| f(req)),
        }
    }
}

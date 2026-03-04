//! Transport-safe DTOs for external GUI/service integration.
//!
//! These types provide a serializable boundary for `InputRequest` and
//! `InputResponse` across process/language boundaries.

use super::{
    InputContext, InputOption, InputRequest, InputRequestKind, InputResponse, InputResponseKind,
    PermissionSuggestion, Question,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRequestDto {
    pub id: String,
    pub kind: InputRequestKindDto,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputRequestKindDto {
    Questions {
        questions: Vec<Question>,
    },
    Permission {
        tool_name: String,
        description: String,
        input: serde_json::Value,
        suggestions: Vec<PermissionSuggestion>,
    },
    FreeText {
        prompt: String,
        last_response: String,
    },
    Simple {
        question: String,
        options: Option<Vec<InputOption>>,
        multi_select: bool,
        context: InputContext,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputResponseDto {
    pub request_id: String,
    pub kind: InputResponseKindDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputResponseKindDto {
    QuestionAnswers {
        answers: HashMap<String, String>,
    },
    PermissionGranted {
        modified_input: Option<serde_json::Value>,
        rules: Vec<PermissionSuggestion>,
    },
    PermissionDenied {
        reason: Option<String>,
    },
    FreeText {
        text: String,
    },
    Cancelled,
    Simple {
        content: String,
        selected_indices: Option<Vec<usize>>,
    },
}

impl From<&InputRequest> for InputRequestDto {
    fn from(request: &InputRequest) -> Self {
        let kind = match &request.kind {
            InputRequestKind::Questions { questions } => InputRequestKindDto::Questions {
                questions: questions.clone(),
            },
            InputRequestKind::Permission {
                tool_name,
                description,
                input,
                suggestions,
            } => InputRequestKindDto::Permission {
                tool_name: tool_name.clone(),
                description: description.clone(),
                input: input.clone(),
                suggestions: suggestions.clone(),
            },
            InputRequestKind::FreeText {
                prompt,
                last_response,
            } => InputRequestKindDto::FreeText {
                prompt: prompt.clone(),
                last_response: last_response.clone(),
            },
            InputRequestKind::Simple {
                question,
                options,
                multi_select,
                context,
            } => InputRequestKindDto::Simple {
                question: question.clone(),
                options: options.clone(),
                multi_select: *multi_select,
                context: *context,
            },
        };

        Self {
            id: request.id.to_string(),
            kind,
            timeout_ms: request
                .timeout
                .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
        }
    }
}

impl From<InputRequest> for InputRequestDto {
    fn from(request: InputRequest) -> Self {
        Self::from(&request)
    }
}

impl TryFrom<InputRequestDto> for InputRequest {
    type Error = String;

    fn try_from(dto: InputRequestDto) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&dto.id).map_err(|e| format!("invalid request id: {}", e))?;
        let kind = match dto.kind {
            InputRequestKindDto::Questions { questions } => {
                InputRequestKind::Questions { questions }
            }
            InputRequestKindDto::Permission {
                tool_name,
                description,
                input,
                suggestions,
            } => InputRequestKind::Permission {
                tool_name,
                description,
                input,
                suggestions,
            },
            InputRequestKindDto::FreeText {
                prompt,
                last_response,
            } => InputRequestKind::FreeText {
                prompt,
                last_response,
            },
            InputRequestKindDto::Simple {
                question,
                options,
                multi_select,
                context,
            } => InputRequestKind::Simple {
                question,
                options,
                multi_select,
                context,
            },
        };

        Ok(InputRequest {
            id,
            kind,
            timeout: dto.timeout_ms.map(Duration::from_millis),
        })
    }
}

impl From<&InputResponse> for InputResponseDto {
    fn from(response: &InputResponse) -> Self {
        let kind = match &response.kind {
            InputResponseKind::QuestionAnswers { answers } => {
                InputResponseKindDto::QuestionAnswers {
                    answers: answers.clone(),
                }
            }
            InputResponseKind::PermissionGranted {
                modified_input,
                rules,
            } => InputResponseKindDto::PermissionGranted {
                modified_input: modified_input.clone(),
                rules: rules.clone(),
            },
            InputResponseKind::PermissionDenied { reason } => {
                InputResponseKindDto::PermissionDenied {
                    reason: reason.clone(),
                }
            }
            InputResponseKind::FreeText { text } => {
                InputResponseKindDto::FreeText { text: text.clone() }
            }
            InputResponseKind::Cancelled => InputResponseKindDto::Cancelled,
            InputResponseKind::Simple {
                content,
                selected_indices,
            } => InputResponseKindDto::Simple {
                content: content.clone(),
                selected_indices: selected_indices.clone(),
            },
        };

        Self {
            request_id: response.request_id.to_string(),
            kind,
        }
    }
}

impl From<InputResponse> for InputResponseDto {
    fn from(response: InputResponse) -> Self {
        Self::from(&response)
    }
}

impl TryFrom<InputResponseDto> for InputResponse {
    type Error = String;

    fn try_from(dto: InputResponseDto) -> Result<Self, Self::Error> {
        let request_id =
            Uuid::parse_str(&dto.request_id).map_err(|e| format!("invalid request_id: {}", e))?;

        let kind = match dto.kind {
            InputResponseKindDto::QuestionAnswers { answers } => {
                InputResponseKind::QuestionAnswers { answers }
            }
            InputResponseKindDto::PermissionGranted {
                modified_input,
                rules,
            } => InputResponseKind::PermissionGranted {
                modified_input,
                rules,
            },
            InputResponseKindDto::PermissionDenied { reason } => {
                InputResponseKind::PermissionDenied { reason }
            }
            InputResponseKindDto::FreeText { text } => InputResponseKind::FreeText { text },
            InputResponseKindDto::Cancelled => InputResponseKind::Cancelled,
            InputResponseKindDto::Simple {
                content,
                selected_indices,
            } => InputResponseKind::Simple {
                content,
                selected_indices,
            },
        };

        Ok(InputResponse { request_id, kind })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_dto_roundtrip() {
        let request =
            InputRequest::free_text("prompt", "last").with_timeout(Duration::from_secs(5));
        let dto = InputRequestDto::from(&request);
        let reconstructed = InputRequest::try_from(dto).expect("request roundtrip");
        assert_eq!(reconstructed.id, request.id);
        assert!(reconstructed.is_free_text());
        assert_eq!(reconstructed.timeout, request.timeout);
    }

    #[test]
    fn response_dto_roundtrip() {
        let response = InputResponse::free_text(Uuid::new_v4(), "ok");
        let dto = InputResponseDto::from(&response);
        let reconstructed = InputResponse::try_from(dto).expect("response roundtrip");
        assert_eq!(reconstructed.request_id, response.request_id);
        assert_eq!(reconstructed.get_text(), Some("ok"));
    }
}

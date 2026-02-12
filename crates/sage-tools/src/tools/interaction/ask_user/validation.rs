//! Validation logic for questions and options

use sage_core::input::Question;
use sage_core::tools::base::ToolError;

/// Validate that questions are well-formed
pub fn validate_questions(questions: &[Question]) -> Result<(), ToolError> {
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
        validate_question(idx, question)?;
    }

    Ok(())
}

/// Validate a single question
fn validate_question(idx: usize, question: &Question) -> Result<(), ToolError> {
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

    Ok(())
}

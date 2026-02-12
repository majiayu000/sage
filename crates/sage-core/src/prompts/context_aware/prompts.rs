//! Phase-specific prompt fragments

use super::ConversationPhase;

/// Phase-specific prompt fragments
pub struct PhasePrompts;

impl PhasePrompts {
    /// Get the prompt fragment for a specific phase
    pub fn for_phase(phase: ConversationPhase) -> &'static str {
        match phase {
            ConversationPhase::Initial => Self::INITIAL,
            ConversationPhase::Exploring => Self::EXPLORING,
            ConversationPhase::Planning => Self::PLANNING,
            ConversationPhase::Implementing => Self::IMPLEMENTING,
            ConversationPhase::Debugging => Self::DEBUGGING,
            ConversationPhase::Testing => Self::TESTING,
            ConversationPhase::Reviewing => Self::REVIEWING,
            ConversationPhase::Completing => Self::COMPLETING,
        }
    }

    /// Initial phase: Understanding the request
    pub const INITIAL: &'static str = r#"## Current Phase: Understanding Request

Focus on understanding what the user needs:
- Read the request carefully before taking action
- If the task is unclear, ask ONE clarifying question
- For clear tasks, start working immediately
- Don't over-plan simple tasks"#;

    /// Exploring phase: Gathering context
    pub const EXPLORING: &'static str = r#"## Current Phase: Exploring Codebase

You are gathering context about the codebase:
- Use ${TASK_TOOL_NAME} with Explore agents for broad searches
- Read relevant files to understand the structure
- Look for existing patterns to follow
- Note dependencies and relationships
- Don't start implementing until you understand the context"#;

    /// Planning phase: Designing approach
    pub const PLANNING: &'static str = r#"## Current Phase: Planning Implementation

You are designing the implementation approach:
- Consider multiple approaches before choosing
- Keep the solution as simple as possible
- Identify files that need to be modified
- Think about edge cases and error handling
- Write your plan to the plan file"#;

    /// Implementing phase: Writing code
    pub const IMPLEMENTING: &'static str = r#"## Current Phase: Implementing Changes

You are actively writing code:
- Follow existing code patterns and style
- Make minimal, focused changes
- Don't over-engineer or add unnecessary features
- Test your changes as you go
- Keep commits atomic and well-described"#;

    /// Debugging phase: Fixing issues
    pub const DEBUGGING: &'static str = r#"## Current Phase: Debugging

You are investigating and fixing issues:
- Read error messages carefully
- Identify the root cause before fixing
- Make minimal changes to fix the issue
- Verify the fix doesn't break other things
- Add tests to prevent regression if appropriate"#;

    /// Testing phase: Verifying behavior
    pub const TESTING: &'static str = r#"## Current Phase: Testing

You are verifying the implementation:
- Run existing tests to check for regressions
- Test the specific functionality you changed
- Check edge cases and error conditions
- Fix any failing tests before moving on"#;

    /// Reviewing phase: Final checks
    pub const REVIEWING: &'static str = r#"## Current Phase: Reviewing

You are doing final checks:
- Review the changes for correctness
- Check for any missed requirements
- Ensure code quality and style consistency
- Verify all tests pass
- Prepare summary of changes made"#;

    /// Completing phase: Wrapping up
    pub const COMPLETING: &'static str = r#"## Current Phase: Completing

You are wrapping up the task:
- Summarize what was accomplished
- Note any remaining items or follow-ups
- Ensure all changes are committed if requested
- Provide clear next steps if applicable"#;

    /// Get a compact reminder for the phase (for injection into messages)
    pub fn compact_reminder(phase: ConversationPhase) -> String {
        match phase {
            ConversationPhase::Initial => {
                "Phase: Initial - Focus on understanding the request".to_string()
            }
            ConversationPhase::Exploring => {
                "Phase: Exploring - Gather context before implementing".to_string()
            }
            ConversationPhase::Planning => {
                "Phase: Planning - Design approach, write to plan file".to_string()
            }
            ConversationPhase::Implementing => {
                "Phase: Implementing - Write minimal, focused code".to_string()
            }
            ConversationPhase::Debugging => {
                "Phase: Debugging - Find root cause, make minimal fix".to_string()
            }
            ConversationPhase::Testing => {
                "Phase: Testing - Verify changes, check for regressions".to_string()
            }
            ConversationPhase::Reviewing => {
                "Phase: Reviewing - Final checks before completion".to_string()
            }
            ConversationPhase::Completing => {
                "Phase: Completing - Summarize and wrap up".to_string()
            }
        }
    }
}

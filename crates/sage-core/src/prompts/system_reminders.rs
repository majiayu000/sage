//! System reminders - Runtime context reminders
//!
//! These are injected into the conversation at runtime to provide
//! context-specific guidance.

use std::fmt;

/// Types of system reminders
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemReminder {
    /// Reminder about todo list status
    TodoListStatus { is_empty: bool, task_count: usize },
    /// Reminder about file operations
    FileOperationWarning { message: String },
    /// Reminder about plan mode phase
    PlanModePhase { phase: PlanPhase, instructions: String },
    /// Reminder about task completion
    TaskCompletionReminder,
    /// Custom reminder
    Custom { title: String, content: String },
}

/// Plan mode phases
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanPhase {
    Understanding,
    Designing,
    Reviewing,
    Finalizing,
    Exiting,
}

impl fmt::Display for PlanPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanPhase::Understanding => write!(f, "Understanding"),
            PlanPhase::Designing => write!(f, "Designing"),
            PlanPhase::Reviewing => write!(f, "Reviewing"),
            PlanPhase::Finalizing => write!(f, "Finalizing"),
            PlanPhase::Exiting => write!(f, "Exiting"),
        }
    }
}

impl SystemReminder {
    /// Convert reminder to string format
    pub fn to_prompt_string(&self) -> String {
        match self {
            SystemReminder::TodoListStatus { is_empty, task_count } => {
                if *is_empty {
                    "<system-reminder>\n\
                    Your todo list is currently empty. If you are working on tasks \
                    that would benefit from tracking, use the TodoWrite tool.\n\
                    </system-reminder>".to_string()
                } else {
                    format!(
                        "<system-reminder>\n\
                        You have {} tasks in your todo list. Remember to update task \
                        status as you complete them.\n\
                        </system-reminder>",
                        task_count
                    )
                }
            }
            SystemReminder::FileOperationWarning { message } => {
                format!(
                    "<system-reminder>\n\
                    WARNING: {}\n\
                    </system-reminder>",
                    message
                )
            }
            SystemReminder::PlanModePhase { phase, instructions } => {
                format!(
                    "<system-reminder>\n\
                    PLAN MODE - Phase: {}\n\n\
                    {}\n\
                    </system-reminder>",
                    phase, instructions
                )
            }
            SystemReminder::TaskCompletionReminder => {
                "<system-reminder>\n\
                REMINDER: Do not call task_done unless you have:\n\
                - Created or modified actual code files\n\
                - Verified the implementation works\n\
                - Completed ALL requested functionality\n\
                </system-reminder>".to_string()
            }
            SystemReminder::Custom { title, content } => {
                format!(
                    "<system-reminder>\n\
                    {}\n\n\
                    {}\n\
                    </system-reminder>",
                    title, content
                )
            }
        }
    }

    /// Create a plan mode reminder for each phase
    pub fn for_plan_phase(phase: PlanPhase) -> Self {
        let instructions = match phase {
            PlanPhase::Understanding => {
                "Focus on understanding requirements:\n\
                - Use Explore agents to investigate the codebase\n\
                - Ask clarifying questions if needed\n\
                - Identify key components and dependencies"
            }
            PlanPhase::Designing => {
                "Design the implementation approach:\n\
                - Consider multiple solutions\n\
                - Evaluate trade-offs\n\
                - Keep it simple and focused"
            }
            PlanPhase::Reviewing => {
                "Review your design:\n\
                - Read critical files to verify assumptions\n\
                - Ask user if uncertain about approach\n\
                - Prepare final plan"
            }
            PlanPhase::Finalizing => {
                "Finalize your plan:\n\
                - Write the plan to the plan file\n\
                - Include specific implementation steps\n\
                - Prepare to exit plan mode"
            }
            PlanPhase::Exiting => {
                "Ready to exit plan mode:\n\
                - Call ExitPlanMode when ready\n\
                - User will review and approve\n\
                - Then START WRITING CODE immediately"
            }
        };

        SystemReminder::PlanModePhase {
            phase,
            instructions: instructions.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_list_reminder_empty() {
        let reminder = SystemReminder::TodoListStatus { is_empty: true, task_count: 0 };
        let text = reminder.to_prompt_string();
        assert!(text.contains("system-reminder"));
        assert!(text.contains("empty"));
    }

    #[test]
    fn test_todo_list_reminder_with_tasks() {
        let reminder = SystemReminder::TodoListStatus { is_empty: false, task_count: 5 };
        let text = reminder.to_prompt_string();
        assert!(text.contains("5 tasks"));
    }

    #[test]
    fn test_plan_phase_reminder() {
        let reminder = SystemReminder::for_plan_phase(PlanPhase::Understanding);
        let text = reminder.to_prompt_string();
        assert!(text.contains("Understanding"));
        assert!(text.contains("Explore agents"));
    }

    #[test]
    fn test_task_completion_reminder() {
        let reminder = SystemReminder::TaskCompletionReminder;
        let text = reminder.to_prompt_string();
        assert!(text.contains("task_done"));
        assert!(text.contains("code files"));
    }
}

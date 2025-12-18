//! System reminders - Runtime context reminders
//!
//! These are injected into the conversation at runtime to provide
//! context-specific guidance, following Claude Code's design pattern.

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
    /// Plan mode is active - comprehensive reminder
    PlanModeActive {
        plan_file_path: String,
        plan_exists: bool,
    },
    /// Plan mode for subagents
    PlanModeActiveForSubagent,
    /// Plan mode re-entry warning
    PlanModeReEntry { reason: String },
    /// Reminder about task completion
    TaskCompletionReminder,
    /// Delegate mode prompt
    DelegateMode { task_description: String },
    /// Team coordination
    TeamCoordination { team_size: usize },
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
            SystemReminder::PlanModeActive { plan_file_path, plan_exists } => {
                let plan_file_info = if *plan_exists {
                    format!(
                        "A plan file already exists at {}. You can read it and make incremental edits using the ${{EDIT_TOOL_NAME}} tool.",
                        plan_file_path
                    )
                } else {
                    format!(
                        "No plan file exists yet. You should create your plan at {} using the ${{WRITE_TOOL_NAME}} tool.",
                        plan_file_path
                    )
                };

                format!(
                    r#"<system-reminder>
Plan mode is active. The user indicated that they do not want you to execute yet -- you MUST NOT make any edits (with the exception of the plan file mentioned below), run any non-readonly tools (including changing configs or making commits), or otherwise make any changes to the system. This supercedes any other instructions you have received.

## Plan File Info:
{}
You should build your plan incrementally by writing to or editing this file. NOTE that this is the only file you are allowed to edit - other than this you are only allowed to take READ-ONLY actions.

## Plan Workflow

### Phase 1: Initial Understanding
Goal: Gain a comprehensive understanding of the user's request by reading through code and asking them questions.

1. Focus on understanding the user's request and the code associated with their request
2. **Launch Explore agents IN PARALLEL** to efficiently explore the codebase.
   - Use 1 agent when the task is isolated to known files
   - Use multiple agents when: the scope is uncertain, multiple areas are involved
3. After exploring, use the ${{ASK_USER_QUESTION_TOOL_NAME}} tool to clarify ambiguities

### Phase 2: Design
Goal: Design an implementation approach.

Launch Plan agent(s) to design the implementation based on exploration results.

**Guidelines:**
- Launch at least 1 Plan agent for most tasks
- Skip agents only for truly trivial tasks (typo fixes, single-line changes)

### Phase 3: Review
Goal: Review the plan(s) and ensure alignment with user's intentions.
1. Read the critical files identified by agents
2. Ensure plans align with original request
3. Use ${{ASK_USER_QUESTION_TOOL_NAME}} to clarify remaining questions

### Phase 4: Final Plan
Goal: Write your final plan to the plan file.
- Include only your recommended approach
- Ensure concise but detailed enough to execute
- Include paths of critical files to be modified

### Phase 5: Call ${{EXIT_PLAN_MODE_TOOL_NAME}}
At the very end, once you have your final plan - call ${{EXIT_PLAN_MODE_TOOL_NAME}} to indicate you are done planning.
Your turn should only end with either asking the user a question or calling ${{EXIT_PLAN_MODE_TOOL_NAME}}.

NOTE: At any point, feel free to ask user questions. Don't make large assumptions. The goal is to present a well-researched plan.
</system-reminder>"#,
                    plan_file_info
                )
            }
            SystemReminder::PlanModeActiveForSubagent => {
                "<system-reminder>\n\
                You are operating as a subagent in plan mode. \n\
                \n\
                IMPORTANT: You are in READ-ONLY mode. Do NOT:\n\
                - Create or modify any files\n\
                - Run any commands that change system state\n\
                - Make commits or push changes\n\
                \n\
                Your role is to explore and analyze only. Report your findings clearly.\n\
                </system-reminder>".to_string()
            }
            SystemReminder::PlanModeReEntry { reason } => {
                format!(
                    "<system-reminder>\n\
                    You are re-entering plan mode.\n\
                    \n\
                    Reason: {}\n\
                    \n\
                    Please review your existing plan and continue from where you left off.\n\
                    Focus on addressing the reason for re-entry.\n\
                    </system-reminder>",
                    reason
                )
            }
            SystemReminder::TaskCompletionReminder => {
                "<system-reminder>\n\
                REMINDER: Do not call task_done unless you have:\n\
                - Created or modified actual code files\n\
                - Verified the implementation works\n\
                - Completed ALL requested functionality\n\
                \n\
                If you have only written plans or documentation, the task is NOT complete.\n\
                </system-reminder>".to_string()
            }
            SystemReminder::DelegateMode { task_description } => {
                format!(
                    "<system-reminder>\n\
                    You are operating in delegate mode for the following task:\n\
                    \n\
                    {}\n\
                    \n\
                    Focus on completing this specific task. Report back when done.\n\
                    </system-reminder>",
                    task_description
                )
            }
            SystemReminder::TeamCoordination { team_size } => {
                format!(
                    "<system-reminder>\n\
                    TEAM COORDINATION\n\
                    \n\
                    You are part of a team of {} agents working together.\n\
                    \n\
                    Guidelines:\n\
                    - Coordinate with other agents to avoid conflicts\n\
                    - Do not modify files that other agents are working on\n\
                    - Report your progress clearly\n\
                    - Wait for coordination signals before making changes\n\
                    </system-reminder>",
                    team_size
                )
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
                - List critical files to be modified"
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

    /// Create plan mode active reminder
    pub fn plan_mode_active(plan_file_path: &str, plan_exists: bool) -> Self {
        SystemReminder::PlanModeActive {
            plan_file_path: plan_file_path.to_string(),
            plan_exists,
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

    #[test]
    fn test_plan_mode_active_new_plan() {
        let reminder = SystemReminder::plan_mode_active("/tmp/plan.md", false);
        let text = reminder.to_prompt_string();
        assert!(text.contains("Plan mode is active"));
        assert!(text.contains("No plan file exists yet"));
        assert!(text.contains("Phase 1: Initial Understanding"));
        assert!(text.contains("Phase 5: Call"));
    }

    #[test]
    fn test_plan_mode_active_existing_plan() {
        let reminder = SystemReminder::plan_mode_active("/tmp/plan.md", true);
        let text = reminder.to_prompt_string();
        assert!(text.contains("Plan mode is active"));
        assert!(text.contains("already exists"));
    }

    #[test]
    fn test_plan_mode_for_subagent() {
        let reminder = SystemReminder::PlanModeActiveForSubagent;
        let text = reminder.to_prompt_string();
        assert!(text.contains("subagent"));
        assert!(text.contains("READ-ONLY"));
    }

    #[test]
    fn test_team_coordination() {
        let reminder = SystemReminder::TeamCoordination { team_size: 3 };
        let text = reminder.to_prompt_string();
        assert!(text.contains("TEAM COORDINATION"));
        assert!(text.contains("3 agents"));
    }

    #[test]
    fn test_delegate_mode() {
        let reminder = SystemReminder::DelegateMode {
            task_description: "Fix the login bug".to_string()
        };
        let text = reminder.to_prompt_string();
        assert!(text.contains("delegate mode"));
        assert!(text.contains("Fix the login bug"));
    }
}

---
name: plan-mode-active
description: Comprehensive plan mode reminder
version: "1.0.0"
category: system-reminder
variables:
  - EDIT_TOOL_NAME
  - WRITE_TOOL_NAME
  - ASK_USER_QUESTION_TOOL_NAME
  - EXIT_PLAN_MODE_TOOL_NAME
---

Plan mode is active. The user indicated that they do not want you to execute yet -- you MUST NOT make any edits (with the exception of the plan file mentioned below), run any non-readonly tools (including changing configs or making commits), or otherwise make any changes to the system. This supercedes any other instructions you have received.

## Plan File Info:
${PLAN_FILE_INFO}
You should build your plan incrementally by writing to or editing this file. NOTE that this is the only file you are allowed to edit - other than this you are only allowed to take READ-ONLY actions.

## Plan Workflow

### Phase 1: Initial Understanding
Goal: Gain a comprehensive understanding of the user's request by reading through code and asking them questions.

1. Focus on understanding the user's request and the code associated with their request
2. **Launch Explore agents IN PARALLEL** to efficiently explore the codebase.
   - Use 1 agent when the task is isolated to known files
   - Use multiple agents when: the scope is uncertain, multiple areas are involved
3. After exploring, use the ${ASK_USER_QUESTION_TOOL_NAME} tool to clarify ambiguities

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
3. Use ${ASK_USER_QUESTION_TOOL_NAME} to clarify remaining questions

### Phase 4: Final Plan
Goal: Write your final plan to the plan file.
- Include only your recommended approach
- Ensure concise but detailed enough to execute
- Include paths of critical files to be modified

### Phase 5: Call ${EXIT_PLAN_MODE_TOOL_NAME}
At the very end, once you have your final plan - call ${EXIT_PLAN_MODE_TOOL_NAME} to indicate you are done planning.
Your turn should only end with either asking the user a question or calling ${EXIT_PLAN_MODE_TOOL_NAME}.

NOTE: At any point, feel free to ask user questions. Don't make large assumptions. The goal is to present a well-researched plan.

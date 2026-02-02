---
name: todo-list-status
description: Todo list status reminder
version: "1.0.0"
category: system-reminder
variables:
  - IS_EMPTY
  - TASK_COUNT
---

${IS_EMPTY?`Your todo list is currently empty. If you are working on tasks that would benefit from tracking, use the TodoWrite tool.`:`You have ${TASK_COUNT} tasks in your todo list. Remember to update task status as you complete them.`}

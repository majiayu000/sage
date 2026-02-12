//! Tests for task management tools

#[cfg(test)]
mod suite {
    use super::super::add_tool::AddTasksTool;
    use super::super::task_list::GLOBAL_TASK_LIST;
    use super::super::types::TaskState;
    use super::super::update_tool::UpdateTasksTool;
    use super::super::view_tool::ViewTasklistTool;
    use sage_core::tools::base::Tool;
    use sage_core::tools::types::ToolCall;
    use serde_json::json;
    use serial_test::serial;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    // Helper function to clear the global task list
    fn clear_global_task_list() {
        GLOBAL_TASK_LIST.tasks.lock().clear();
        GLOBAL_TASK_LIST.root_tasks.lock().clear();
    }

    #[tokio::test]
    #[serial]
    async fn test_view_empty_tasklist() {
        clear_global_task_list();

        let tool = ViewTasklistTool::new();
        let call = create_tool_call("test-1", "ViewTasklist", json!({}));

        let result = tool.execute(&call).await.unwrap();
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("No tasks in the current task list"));
    }

    #[tokio::test]
    #[serial]
    async fn test_add_single_task() {
        clear_global_task_list();

        let tool = AddTasksTool::new();
        let call = create_tool_call(
            "test-2",
            "AddTasks",
            json!({
                "tasks": [{
                    "name": "Test Task",
                    "description": "This is a test task",
                    "state": "NOT_STARTED"
                }]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(
            result
                .output
                .as_ref()
                .unwrap()
                .contains("Successfully created 1 task(s)")
        );

        // Verify task was added
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "ViewTasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("Test Task"));
        assert!(
            view_result
                .output
                .as_ref()
                .unwrap()
                .contains("This is a test task")
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_add_multiple_tasks() {
        clear_global_task_list();

        let tool = AddTasksTool::new();
        let call = create_tool_call(
            "test-3",
            "AddTasks",
            json!({
                "tasks": [
                    {
                        "name": "Task 1",
                        "description": "First task",
                        "state": "NOT_STARTED"
                    },
                    {
                        "name": "Task 2",
                        "description": "Second task",
                        "state": "IN_PROGRESS"
                    }
                ]
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(
            result
                .output
                .as_ref()
                .unwrap()
                .contains("Successfully created 2 task(s)")
        );

        // Verify both tasks were added
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "ViewTasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("Task 1"));
        assert!(view_result.output.as_ref().unwrap().contains("Task 2"));
        assert!(view_result.output.as_ref().unwrap().contains("[ ]")); // NOT_STARTED
        assert!(view_result.output.as_ref().unwrap().contains("[/]")); // IN_PROGRESS
    }

    #[tokio::test]
    #[serial]
    async fn test_update_task_state() {
        clear_global_task_list();

        // Add a task first
        let add_tool = AddTasksTool::new();
        let add_call = create_tool_call(
            "test-add",
            "AddTasks",
            json!({
                "tasks": [{
                    "name": "Update Test Task",
                    "description": "Task to be updated",
                    "state": "NOT_STARTED"
                }]
            }),
        );
        add_tool.execute(&add_call).await.unwrap();

        // Get the task ID
        let task_id = {
            let _tasks = GLOBAL_TASK_LIST.tasks.lock();
            let root_tasks = GLOBAL_TASK_LIST.root_tasks.lock();
            root_tasks
                .first()
                .expect("Task list should not be empty after adding a task")
                .clone()
        };

        // Update the task
        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call(
            "test-update",
            "UpdateTasks",
            json!({
                "tasks": [{
                    "task_id": task_id,
                    "state": "COMPLETE"
                }]
            }),
        );

        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 1"));

        // Verify the update
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "ViewTasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("[x]")); // COMPLETE
    }

    #[tokio::test]
    #[serial]
    async fn test_update_multiple_tasks() {
        clear_global_task_list();

        // Add multiple tasks
        let add_tool = AddTasksTool::new();
        let add_call = create_tool_call(
            "test-add",
            "AddTasks",
            json!({
                "tasks": [
                    {
                        "name": "Task A",
                        "description": "First task",
                        "state": "NOT_STARTED"
                    },
                    {
                        "name": "Task B",
                        "description": "Second task",
                        "state": "NOT_STARTED"
                    }
                ]
            }),
        );
        add_tool.execute(&add_call).await.unwrap();

        // Get task IDs
        let (task_id_1, task_id_2) = {
            let root_tasks = GLOBAL_TASK_LIST.root_tasks.lock();
            (root_tasks[0].clone(), root_tasks[1].clone())
        };

        // Update both tasks
        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call(
            "test-update",
            "UpdateTasks",
            json!({
                "tasks": [
                    {
                        "task_id": task_id_1,
                        "state": "COMPLETE"
                    },
                    {
                        "task_id": task_id_2,
                        "state": "IN_PROGRESS"
                    }
                ]
            }),
        );

        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 2"));

        // Verify the updates
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("test-view", "ViewTasklist", json!({}));
        let view_result = view_tool.execute(&view_call).await.unwrap();
        assert!(view_result.output.as_ref().unwrap().contains("[x]")); // COMPLETE
        assert!(view_result.output.as_ref().unwrap().contains("[/]")); // IN_PROGRESS
    }

    #[tokio::test]
    #[serial]
    async fn test_update_nonexistent_task() {
        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call(
            "test-update",
            "UpdateTasks",
            json!({
                "tasks": [{
                    "task_id": "nonexistent-id",
                    "state": "COMPLETE"
                }]
            }),
        );

        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Errors:"));
        assert!(result.output.as_ref().unwrap().contains("Task not found"));
    }

    #[tokio::test]
    #[serial]
    async fn test_task_state_display() {
        assert_eq!(format!("{}", TaskState::NotStarted), "[ ]");
        assert_eq!(format!("{}", TaskState::InProgress), "[/]");
        assert_eq!(format!("{}", TaskState::Cancelled), "[-]");
        assert_eq!(format!("{}", TaskState::Complete), "[x]");
    }

    #[tokio::test]
    #[serial]
    async fn test_full_workflow_integration() {
        clear_global_task_list();

        // Step 1: View empty task list
        let view_tool = ViewTasklistTool::new();
        let view_call = create_tool_call("view-1", "ViewTasklist", json!({}));
        let result = view_tool.execute(&view_call).await.unwrap();
        assert!(
            result
                .output
                .as_ref()
                .unwrap()
                .contains("No tasks in the current task list")
        );

        // Step 2: Add some tasks
        let add_tool = AddTasksTool::new();
        let add_call = create_tool_call(
            "add-1",
            "AddTasks",
            json!({
                "tasks": [
                    {
                        "name": "Setup Project",
                        "description": "Initialize the project structure",
                        "state": "NOT_STARTED"
                    },
                    {
                        "name": "Implement Core Features",
                        "description": "Build the main functionality",
                        "state": "NOT_STARTED"
                    },
                    {
                        "name": "Write Tests",
                        "description": "Create comprehensive test suite",
                        "state": "NOT_STARTED"
                    }
                ]
            }),
        );
        let result = add_tool.execute(&add_call).await.unwrap();
        assert!(
            result
                .output
                .as_ref()
                .unwrap()
                .contains("Successfully created 3 task(s)")
        );

        // Step 3: View the task list with tasks
        let view_call = create_tool_call("view-2", "ViewTasklist", json!({}));
        let result = view_tool.execute(&view_call).await.unwrap();
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Setup Project"));
        assert!(output.contains("Implement Core Features"));
        assert!(output.contains("Write Tests"));
        assert!(output.contains("[ ]")); // All should be NOT_STARTED

        // Step 4: Start working on first task
        let task_ids = {
            let root_tasks = GLOBAL_TASK_LIST.root_tasks.lock();
            root_tasks.clone()
        };

        let update_tool = UpdateTasksTool::new();
        let update_call = create_tool_call(
            "update-1",
            "UpdateTasks",
            json!({
                "tasks": [{
                    "task_id": task_ids[0],
                    "state": "IN_PROGRESS"
                }]
            }),
        );
        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 1"));

        // Step 5: Complete first task and start second
        let update_call = create_tool_call(
            "update-2",
            "UpdateTasks",
            json!({
                "tasks": [
                    {
                        "task_id": task_ids[0],
                        "state": "COMPLETE"
                    },
                    {
                        "task_id": task_ids[1],
                        "state": "IN_PROGRESS"
                    }
                ]
            }),
        );
        let result = update_tool.execute(&update_call).await.unwrap();
        assert!(result.output.as_ref().unwrap().contains("Updated: 2"));

        // Step 6: View final state
        let view_call = create_tool_call("view-3", "ViewTasklist", json!({}));
        let result = view_tool.execute(&view_call).await.unwrap();
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("[x]")); // COMPLETE
        assert!(output.contains("[/]")); // IN_PROGRESS
        assert!(output.contains("[ ]")); // NOT_STARTED (third task)
    }
}

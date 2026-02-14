use super::*;
use serde_json::json;

#[tokio::test]
async fn test_todo_write_basic() {
    let tool = TodoWriteTool::with_list(Arc::new(TodoList::new()));

    let call = ToolCall {
        id: "test-1".to_string(),
        name: "TodoWrite".to_string(),
        arguments: json!({
            "todos": [
                {
                    "content": "Implement feature A",
                    "status": "in_progress",
                    "activeForm": "Implementing feature A"
                },
                {
                    "content": "Write tests",
                    "status": "pending",
                    "activeForm": "Writing tests"
                }
            ]
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("2 total"));
    assert!(output.contains("1 in progress"));
}

#[tokio::test]
async fn test_todo_write_completion() {
    let list = Arc::new(TodoList::new());
    let tool = TodoWriteTool::with_list(list.clone());

    // Add initial todos
    let call = ToolCall {
        id: "test-1".to_string(),
        name: "TodoWrite".to_string(),
        arguments: json!({
            "todos": [
                {
                    "content": "Task 1",
                    "status": "completed",
                    "activeForm": "Completing task 1"
                },
                {
                    "content": "Task 2",
                    "status": "in_progress",
                    "activeForm": "Working on task 2"
                },
                {
                    "content": "Task 3",
                    "status": "pending",
                    "activeForm": "Starting task 3"
                }
            ]
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };

    let result = tool.execute(&call).await.unwrap();
    assert!(result.success);
    let output = result.output.unwrap();
    assert!(output.contains("1 completed"));

    let (total, completed, in_progress) = list.get_stats();
    assert_eq!(total, 3);
    assert_eq!(completed, 1);
    assert_eq!(in_progress, 1);
}

#[tokio::test]
async fn test_todo_write_empty_content_error() {
    let tool = TodoWriteTool::with_list(Arc::new(TodoList::new()));

    let call = ToolCall {
        id: "test-1".to_string(),
        name: "TodoWrite".to_string(),
        arguments: json!({
            "todos": [
                {
                    "content": "",
                    "status": "pending",
                    "activeForm": "Doing something"
                }
            ]
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect(),
        call_id: None,
    };

    let result = tool.execute(&call).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_todo_display_format() {
    let list = TodoList::new();
    list.set_todos(vec![
        TodoItem {
            content: "Task 1".to_string(),
            status: TodoStatus::Completed,
            active_form: "Completing task 1".to_string(),
        },
        TodoItem {
            content: "Task 2".to_string(),
            status: TodoStatus::InProgress,
            active_form: "Working on task 2".to_string(),
        },
        TodoItem {
            content: "Task 3".to_string(),
            status: TodoStatus::Pending,
            active_form: "Starting task 3".to_string(),
        },
    ]);

    let display = list.format_display();
    assert!(display.contains("[x] Task 1"));
    assert!(display.contains("[/] Task 2"));
    assert!(display.contains("[ ] Task 3"));
}

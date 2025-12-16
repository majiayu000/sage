//! Example demonstrating the AskUserQuestion tool
//!
//! This example shows how the agent can ask users questions during execution
//! to gather information, clarify requirements, or get decisions.

use sage_core::tools::Tool;
use sage_core::tools::types::ToolCall;
use sage_tools::AskUserQuestionTool;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== AskUserQuestion Tool Demo ===\n");

    let tool = AskUserQuestionTool::new();

    // Example 1: Single question
    println!("Example 1: Single choice question");
    println!("-----------------------------------");

    let mut args = HashMap::new();
    args.insert(
        "questions".to_string(),
        json!([{
            "question": "Which authentication method should we use for the API?",
            "header": "Auth method",
            "options": [
                {
                    "label": "OAuth 2.0",
                    "description": "Industry standard OAuth 2.0 authentication with token-based flow"
                },
                {
                    "label": "JWT",
                    "description": "JSON Web Tokens for stateless authentication"
                },
                {
                    "label": "API Keys",
                    "description": "Simple API key authentication for service-to-service communication"
                }
            ],
            "multi_select": false
        }]),
    );

    let call = ToolCall {
        id: "call-1".to_string(),
        name: tool.name().to_string(),
        arguments: args,
        call_id: None,
    };

    let result = tool.execute(&call).await?;
    println!("{}\n", result.output.unwrap());

    // Example 2: Multiple questions
    println!("Example 2: Multiple questions");
    println!("------------------------------");

    let mut args2 = HashMap::new();
    args2.insert(
        "questions".to_string(),
        json!([
            {
                "question": "Which frontend framework should we use?",
                "header": "Framework",
                "options": [
                    {
                        "label": "React",
                        "description": "Popular component-based library with large ecosystem"
                    },
                    {
                        "label": "Vue.js",
                        "description": "Progressive framework with gentle learning curve"
                    }
                ]
            },
            {
                "question": "Which state management solution?",
                "header": "State mgmt",
                "options": [
                    {
                        "label": "Redux",
                        "description": "Predictable state container with time-travel debugging"
                    },
                    {
                        "label": "MobX",
                        "description": "Simple, scalable state management with reactive programming"
                    }
                ]
            }
        ]),
    );

    let call2 = ToolCall {
        id: "call-2".to_string(),
        name: tool.name().to_string(),
        arguments: args2,
        call_id: None,
    };

    let result2 = tool.execute(&call2).await?;
    println!("{}\n", result2.output.unwrap());

    // Example 3: Multi-select question
    println!("Example 3: Multi-select question");
    println!("--------------------------------");

    let mut args3 = HashMap::new();
    args3.insert(
        "questions".to_string(),
        json!([{
            "question": "Which features should we prioritize for the MVP?",
            "header": "Features",
            "options": [
                {
                    "label": "User authentication",
                    "description": "Login, registration, and password reset"
                },
                {
                    "label": "Dark mode",
                    "description": "Support for light and dark color schemes"
                },
                {
                    "label": "i18n",
                    "description": "Internationalization and localization support"
                },
                {
                    "label": "Analytics",
                    "description": "Usage tracking and analytics dashboard"
                }
            ],
            "multi_select": true
        }]),
    );

    let call3 = ToolCall {
        id: "call-3".to_string(),
        name: tool.name().to_string(),
        arguments: args3,
        call_id: None,
    };

    let result3 = tool.execute(&call3).await?;
    println!("{}\n", result3.output.unwrap());

    println!("=== Demo Complete ===");

    Ok(())
}

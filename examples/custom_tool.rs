//! Example of creating and using a custom tool

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::executor::ToolExecutorBuilder;
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use sage_sdk::SageAgentSdk;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// A custom tool that calculates mathematical expressions
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Perform basic mathematical calculations. Supports +, -, *, / operations."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "expression",
                "Mathematical expression to evaluate (e.g., '2 + 3 * 4')",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let expression = call.get_string("expression").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'expression' parameter".to_string())
        })?;

        // Simple expression evaluator (for demo purposes)
        let result = match self.evaluate_expression(&expression) {
            Ok(value) => ToolResult::success(
                &call.id,
                self.name(),
                format!("The result of '{}' is: {}", expression, value),
            ),
            Err(e) => ToolResult::error(
                &call.id,
                self.name(),
                format!("Failed to evaluate expression '{}': {}", expression, e),
            ),
        };

        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let expression = call.get_string("expression").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'expression' parameter".to_string())
        })?;

        if expression.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Expression cannot be empty".to_string(),
            ));
        }

        // Basic validation - check for allowed characters
        let allowed_chars = "0123456789+-*/.() ";
        for ch in expression.chars() {
            if !allowed_chars.contains(ch) {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid character '{}' in expression",
                    ch
                )));
            }
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(5)) // 5 seconds max
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Math operations are safe to run in parallel
    }
}

impl CalculatorTool {
    /// Simple expression evaluator (very basic implementation)
    fn evaluate_expression(&self, expr: &str) -> Result<f64, String> {
        // Remove whitespace
        let expr = expr.replace(' ', "");

        // For this example, we'll handle very simple cases
        // In a real implementation, you'd use a proper expression parser

        if let Some(pos) = expr.find('+') {
            let (left, right) = expr.split_at(pos);
            let right = &right[1..]; // Skip the '+'
            let left_val = left.parse::<f64>().map_err(|_| "Invalid left operand")?;
            let right_val = right.parse::<f64>().map_err(|_| "Invalid right operand")?;
            return Ok(left_val + right_val);
        }

        if let Some(pos) = expr.find('-') {
            let (left, right) = expr.split_at(pos);
            let right = &right[1..]; // Skip the '-'
            let left_val = left.parse::<f64>().map_err(|_| "Invalid left operand")?;
            let right_val = right.parse::<f64>().map_err(|_| "Invalid right operand")?;
            return Ok(left_val - right_val);
        }

        if let Some(pos) = expr.find('*') {
            let (left, right) = expr.split_at(pos);
            let right = &right[1..]; // Skip the '*'
            let left_val = left.parse::<f64>().map_err(|_| "Invalid left operand")?;
            let right_val = right.parse::<f64>().map_err(|_| "Invalid right operand")?;
            return Ok(left_val * right_val);
        }

        if let Some(pos) = expr.find('/') {
            let (left, right) = expr.split_at(pos);
            let right = &right[1..]; // Skip the '/'
            let left_val = left.parse::<f64>().map_err(|_| "Invalid left operand")?;
            let right_val = right.parse::<f64>().map_err(|_| "Invalid right operand")?;
            if right_val == 0.0 {
                return Err("Division by zero".to_string());
            }
            return Ok(left_val / right_val);
        }

        // If no operator found, try to parse as a single number
        expr.parse::<f64>()
            .map_err(|_| "Invalid number".to_string())
    }
}

/// A custom tool that generates random numbers
pub struct RandomNumberTool;

#[async_trait]
impl Tool for RandomNumberTool {
    fn name(&self) -> &str {
        "random_number"
    }

    fn description(&self) -> &str {
        "Generate a random number within a specified range."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::number("min", "Minimum value (inclusive)"),
                ToolParameter::number("max", "Maximum value (inclusive)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let min = call
            .get_number("min")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'min' parameter".to_string()))?;

        let max = call
            .get_number("max")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'max' parameter".to_string()))?;

        if min > max {
            return Ok(ToolResult::error(
                &call.id,
                self.name(),
                "Minimum value cannot be greater than maximum value",
            ));
        }

        // Generate random number (using a simple method for demo)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        let hash = hasher.finish();

        let range = max - min;
        let random_value = min + (hash as f64 % range);

        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!(
                "Random number between {} and {}: {:.2}",
                min, max, random_value
            ),
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔧 Custom Tool Example");
    println!("======================");

    // Create a custom tool executor with our tools
    let tool_executor = ToolExecutorBuilder::new()
        .with_tool(Arc::new(CalculatorTool))
        .with_tool(Arc::new(RandomNumberTool))
        // Add default tools as well
        .with_tools(sage_tools::get_default_tools())
        .build();

    // Create SDK with custom configuration
    let _sdk = SageAgentSdk::new()?
        .with_provider_and_model("openai", "gpt-4", None)?
        .with_working_directory("./examples");

    // Note: In a real implementation, you would need to modify the SDK
    // to accept a custom tool executor. For this example, we'll show
    // how the tools would work independently.

    println!("\n🧮 Testing Calculator Tool...");

    // Test the calculator tool directly
    let calc_tool = CalculatorTool;
    let mut call_args = HashMap::new();
    call_args.insert(
        "expression".to_string(),
        serde_json::Value::String("10 + 5".to_string()),
    );

    let tool_call = sage_core::tools::types::ToolCall {
        id: "test-1".to_string(),
        name: "calculator".to_string(),
        arguments: call_args,
        call_id: None,
    };

    match calc_tool.execute(&tool_call).await {
        Ok(result) => {
            println!(
                "✅ Calculator result: {}",
                result.output.unwrap_or_default()
            );
        }
        Err(e) => {
            println!("❌ Calculator error: {}", e);
        }
    }

    println!("\n🎲 Testing Random Number Tool...");

    // Test the random number tool
    let random_tool = RandomNumberTool;
    let mut call_args = HashMap::new();
    call_args.insert(
        "min".to_string(),
        serde_json::Value::Number(serde_json::Number::from(1)),
    );
    call_args.insert(
        "max".to_string(),
        serde_json::Value::Number(serde_json::Number::from(100)),
    );

    let tool_call = sage_core::tools::types::ToolCall {
        id: "test-2".to_string(),
        name: "random_number".to_string(),
        arguments: call_args,
        call_id: None,
    };

    match random_tool.execute(&tool_call).await {
        Ok(result) => {
            println!(
                "✅ Random number result: {}",
                result.output.unwrap_or_default()
            );
        }
        Err(e) => {
            println!("❌ Random number error: {}", e);
        }
    }

    println!("\n📊 Tool Executor Statistics:");
    let stats = tool_executor.get_statistics();
    println!("   Total tools: {}", stats.total_tools);
    println!("   Tool names: {:?}", stats.tool_names);

    println!("\n🎉 Custom tool example completed!");
    Ok(())
}

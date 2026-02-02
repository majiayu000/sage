//! Core metric types for evaluation
//!
//! Defines the data structures for tracking evaluation results and metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::tasks::{Difficulty, TaskCategory};

/// Status of a task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task completed successfully
    Passed,
    /// Task failed verification
    Failed,
    /// Task timed out
    Timeout,
    /// Task encountered an error
    Error,
    /// Task was skipped
    Skipped,
}

impl TaskStatus {
    /// Check if the status represents success
    pub fn is_success(&self) -> bool {
        matches!(self, TaskStatus::Passed)
    }
}

/// Result of a single task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,

    /// Task name
    pub task_name: String,

    /// Task category
    pub category: TaskCategory,

    /// Task difficulty
    pub difficulty: Difficulty,

    /// Execution status
    pub status: TaskStatus,

    /// Number of turns/steps taken
    pub turns: u32,

    /// Total tokens used (input + output)
    pub total_tokens: u64,

    /// Input tokens used
    pub input_tokens: u64,

    /// Output tokens used
    pub output_tokens: u64,

    /// Execution time in seconds
    pub execution_time_secs: f64,

    /// Attempt number (1-indexed)
    pub attempt: u32,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Verifier output/details
    pub verifier_output: Option<String>,

    /// Timestamp of execution
    pub timestamp: DateTime<Utc>,

    /// Tool usage counts
    #[serde(default)]
    pub tool_usage: HashMap<String, u32>,
}

impl TaskResult {
    /// Create a new task result
    pub fn new(
        task_id: impl Into<String>,
        task_name: impl Into<String>,
        category: TaskCategory,
        difficulty: Difficulty,
        status: TaskStatus,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            task_name: task_name.into(),
            category,
            difficulty,
            status,
            turns: 0,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            execution_time_secs: 0.0,
            attempt: 1,
            error_message: None,
            verifier_output: None,
            timestamp: Utc::now(),
            tool_usage: HashMap::new(),
        }
    }

    /// Check if the task passed
    pub fn passed(&self) -> bool {
        self.status.is_success()
    }
}

/// Pass@K metric - probability of passing within K attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassAtK {
    /// K value (number of attempts)
    pub k: u32,

    /// Number of tasks that passed within K attempts
    pub passed: u32,

    /// Total number of tasks
    pub total: u32,

    /// Pass rate (passed / total)
    pub rate: f64,
}

impl PassAtK {
    /// Create a new Pass@K metric
    pub fn new(k: u32, passed: u32, total: u32) -> Self {
        let rate = if total > 0 {
            passed as f64 / total as f64
        } else {
            0.0
        };

        Self {
            k,
            passed,
            total,
            rate,
        }
    }

    /// Format as percentage string
    pub fn as_percentage(&self) -> String {
        format!("{:.1}%", self.rate * 100.0)
    }
}

/// Token efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEfficiency {
    /// Average tokens per successful task
    pub avg_tokens_per_success: f64,

    /// Average tokens per turn
    pub avg_tokens_per_turn: f64,

    /// Total tokens used
    pub total_tokens: u64,

    /// Total input tokens
    pub total_input_tokens: u64,

    /// Total output tokens
    pub total_output_tokens: u64,

    /// Input/output ratio
    pub input_output_ratio: f64,
}

/// Cost estimate for token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Provider name
    pub provider: String,

    /// Model name
    pub model: String,

    /// Input token cost per 1M tokens (USD)
    pub input_cost_per_million: f64,

    /// Output token cost per 1M tokens (USD)
    pub output_cost_per_million: f64,

    /// Total input tokens
    pub input_tokens: u64,

    /// Total output tokens
    pub output_tokens: u64,

    /// Estimated input cost (USD)
    pub input_cost_usd: f64,

    /// Estimated output cost (USD)
    pub output_cost_usd: f64,

    /// Total estimated cost (USD)
    pub total_cost_usd: f64,
}

impl CostEstimate {
    /// Create a cost estimate from token counts and pricing
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        input_tokens: u64,
        output_tokens: u64,
        input_cost_per_million: f64,
        output_cost_per_million: f64,
    ) -> Self {
        let input_cost_usd = (input_tokens as f64 / 1_000_000.0) * input_cost_per_million;
        let output_cost_usd = (output_tokens as f64 / 1_000_000.0) * output_cost_per_million;

        Self {
            provider: provider.into(),
            model: model.into(),
            input_cost_per_million,
            output_cost_per_million,
            input_tokens,
            output_tokens,
            input_cost_usd,
            output_cost_usd,
            total_cost_usd: input_cost_usd + output_cost_usd,
        }
    }

    /// Create cost estimate using default pricing for known providers/models
    pub fn from_provider(provider: &str, model: &str, input_tokens: u64, output_tokens: u64) -> Self {
        let (input_price, output_price) = Self::get_pricing(provider, model);
        Self::new(provider, model, input_tokens, output_tokens, input_price, output_price)
    }

    /// Get pricing for known providers/models (per 1M tokens in USD)
    /// Prices as of early 2025
    fn get_pricing(provider: &str, model: &str) -> (f64, f64) {
        match provider.to_lowercase().as_str() {
            "anthropic" => match model {
                m if m.contains("opus") => (15.0, 75.0),      // Claude Opus
                m if m.contains("sonnet") => (3.0, 15.0),     // Claude Sonnet
                m if m.contains("haiku") => (0.25, 1.25),     // Claude Haiku
                _ => (3.0, 15.0),                              // Default to Sonnet pricing
            },
            "openai" => match model {
                m if m.contains("gpt-4o") => (2.5, 10.0),     // GPT-4o
                m if m.contains("gpt-4-turbo") => (10.0, 30.0), // GPT-4 Turbo
                m if m.contains("gpt-4") => (30.0, 60.0),     // GPT-4
                m if m.contains("gpt-3.5") => (0.5, 1.5),     // GPT-3.5
                m if m.contains("o1") => (15.0, 60.0),        // o1
                _ => (2.5, 10.0),                              // Default to GPT-4o pricing
            },
            "google" => match model {
                m if m.contains("gemini-1.5-pro") => (1.25, 5.0),
                m if m.contains("gemini-1.5-flash") => (0.075, 0.3),
                m if m.contains("gemini-2") => (0.1, 0.4),
                _ => (1.25, 5.0),
            },
            "deepseek" => (0.14, 0.28),                        // DeepSeek V3
            "zhipu" | "glm" => (0.5, 0.5),                     // GLM-4
            _ => (1.0, 1.0),                                    // Unknown provider fallback
        }
    }

    /// Format cost as string
    pub fn format_cost(&self) -> String {
        if self.total_cost_usd < 0.01 {
            format!("${:.4}", self.total_cost_usd)
        } else if self.total_cost_usd < 1.0 {
            format!("${:.3}", self.total_cost_usd)
        } else {
            format!("${:.2}", self.total_cost_usd)
        }
    }

    /// Format detailed cost breakdown
    pub fn format_detailed(&self) -> String {
        format!(
            "Input: {} tokens × ${:.2}/1M = ${:.4}\n\
             Output: {} tokens × ${:.2}/1M = ${:.4}\n\
             Total: {}",
            self.input_tokens,
            self.input_cost_per_million,
            self.input_cost_usd,
            self.output_tokens,
            self.output_cost_per_million,
            self.output_cost_usd,
            self.format_cost()
        )
    }
}

impl TokenEfficiency {
    /// Create from task results
    pub fn from_results(results: &[TaskResult]) -> Self {
        let total_tokens: u64 = results.iter().map(|r| r.total_tokens).sum();
        let total_input: u64 = results.iter().map(|r| r.input_tokens).sum();
        let total_output: u64 = results.iter().map(|r| r.output_tokens).sum();
        let total_turns: u32 = results.iter().map(|r| r.turns).sum();

        let successful: Vec<_> = results.iter().filter(|r| r.passed()).collect();
        let success_tokens: u64 = successful.iter().map(|r| r.total_tokens).sum();

        let avg_tokens_per_success = if !successful.is_empty() {
            success_tokens as f64 / successful.len() as f64
        } else {
            0.0
        };

        let avg_tokens_per_turn = if total_turns > 0 {
            total_tokens as f64 / total_turns as f64
        } else {
            0.0
        };

        let input_output_ratio = if total_output > 0 {
            total_input as f64 / total_output as f64
        } else {
            0.0
        };

        Self {
            avg_tokens_per_success,
            avg_tokens_per_turn,
            total_tokens,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            input_output_ratio,
        }
    }
}

/// Turn/step metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnMetrics {
    /// Average turns per task
    pub avg_turns: f64,

    /// Average turns for successful tasks
    pub avg_turns_success: f64,

    /// Minimum turns
    pub min_turns: u32,

    /// Maximum turns
    pub max_turns: u32,

    /// Total turns across all tasks
    pub total_turns: u32,
}

impl TurnMetrics {
    /// Create from task results
    pub fn from_results(results: &[TaskResult]) -> Self {
        if results.is_empty() {
            return Self {
                avg_turns: 0.0,
                avg_turns_success: 0.0,
                min_turns: 0,
                max_turns: 0,
                total_turns: 0,
            };
        }

        let total_turns: u32 = results.iter().map(|r| r.turns).sum();
        let min_turns = results.iter().map(|r| r.turns).min().unwrap_or(0);
        let max_turns = results.iter().map(|r| r.turns).max().unwrap_or(0);

        let avg_turns = total_turns as f64 / results.len() as f64;

        let successful: Vec<_> = results.iter().filter(|r| r.passed()).collect();
        let success_turns: u32 = successful.iter().map(|r| r.turns).sum();
        let avg_turns_success = if !successful.is_empty() {
            success_turns as f64 / successful.len() as f64
        } else {
            0.0
        };

        Self {
            avg_turns,
            avg_turns_success,
            min_turns,
            max_turns,
            total_turns,
        }
    }
}

/// Metrics for a specific category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMetrics {
    /// Category
    pub category: TaskCategory,

    /// Number of tasks
    pub task_count: u32,

    /// Pass rate
    pub pass_rate: f64,

    /// Average turns
    pub avg_turns: f64,

    /// Average tokens
    pub avg_tokens: f64,

    /// Results by difficulty
    pub by_difficulty: HashMap<String, DifficultyMetrics>,
}

/// Metrics for a specific difficulty level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyMetrics {
    /// Number of tasks
    pub task_count: u32,

    /// Number passed
    pub passed: u32,

    /// Pass rate
    pub pass_rate: f64,
}

/// Aggregated evaluation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalMetrics {
    /// Pass@1 rate (first attempt success)
    pub pass_at_1: PassAtK,

    /// Pass@3 rate (success within 3 attempts)
    pub pass_at_3: Option<PassAtK>,

    /// Token efficiency metrics
    pub token_efficiency: TokenEfficiency,

    /// Turn metrics
    pub turn_metrics: TurnMetrics,

    /// Cost estimate
    pub cost_estimate: CostEstimate,

    /// Metrics by category
    pub by_category: HashMap<String, CategoryMetrics>,

    /// All task results
    pub task_results: Vec<TaskResult>,

    /// Total execution time
    pub total_execution_time_secs: f64,

    /// Evaluation timestamp
    pub timestamp: DateTime<Utc>,

    /// Model used for evaluation
    pub model: String,

    /// Provider used
    pub provider: String,

    /// Sage version
    pub sage_version: String,
}

impl EvalMetrics {
    /// Get overall pass rate
    pub fn overall_pass_rate(&self) -> f64 {
        self.pass_at_1.rate
    }

    /// Get number of passed tasks
    pub fn passed_count(&self) -> u32 {
        self.pass_at_1.passed
    }

    /// Get total task count
    pub fn total_count(&self) -> u32 {
        self.pass_at_1.total
    }

    /// Get failed task count
    pub fn failed_count(&self) -> u32 {
        self.pass_at_1.total - self.pass_at_1.passed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_at_k() {
        let pass_at_1 = PassAtK::new(1, 8, 10);
        assert_eq!(pass_at_1.rate, 0.8);
        assert_eq!(pass_at_1.as_percentage(), "80.0%");
    }

    #[test]
    fn test_task_result() {
        let result = TaskResult::new(
            "test-001",
            "Test Task",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            TaskStatus::Passed,
        );

        assert!(result.passed());
        assert_eq!(result.task_id, "test-001");
    }

    #[test]
    fn test_token_efficiency() {
        let results = vec![
            {
                let mut r = TaskResult::new(
                    "t1",
                    "Task 1",
                    TaskCategory::CodeGeneration,
                    Difficulty::Easy,
                    TaskStatus::Passed,
                );
                r.total_tokens = 1000;
                r.input_tokens = 800;
                r.output_tokens = 200;
                r.turns = 5;
                r
            },
            {
                let mut r = TaskResult::new(
                    "t2",
                    "Task 2",
                    TaskCategory::CodeGeneration,
                    Difficulty::Easy,
                    TaskStatus::Passed,
                );
                r.total_tokens = 2000;
                r.input_tokens = 1600;
                r.output_tokens = 400;
                r.turns = 10;
                r
            },
        ];

        let efficiency = TokenEfficiency::from_results(&results);
        assert_eq!(efficiency.total_tokens, 3000);
        assert_eq!(efficiency.avg_tokens_per_success, 1500.0);
        assert_eq!(efficiency.avg_tokens_per_turn, 200.0);
    }

    #[test]
    fn test_turn_metrics() {
        let results = vec![
            {
                let mut r = TaskResult::new(
                    "t1",
                    "Task 1",
                    TaskCategory::CodeGeneration,
                    Difficulty::Easy,
                    TaskStatus::Passed,
                );
                r.turns = 3;
                r
            },
            {
                let mut r = TaskResult::new(
                    "t2",
                    "Task 2",
                    TaskCategory::CodeGeneration,
                    Difficulty::Easy,
                    TaskStatus::Failed,
                );
                r.turns = 10;
                r
            },
        ];

        let metrics = TurnMetrics::from_results(&results);
        assert_eq!(metrics.min_turns, 3);
        assert_eq!(metrics.max_turns, 10);
        assert_eq!(metrics.total_turns, 13);
        assert_eq!(metrics.avg_turns, 6.5);
        assert_eq!(metrics.avg_turns_success, 3.0);
    }
}

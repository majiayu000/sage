//! Trajectory compression demonstration
//!
//! This example shows how to use trajectory file compression to reduce disk space usage.

use sage_core::error::SageResult;
use sage_core::trajectory::recorder::{
    AgentStepRecord, LLMInteractionRecord, LLMResponseRecord, TokenUsageRecord, TrajectoryRecord,
};
use sage_core::trajectory::storage::{FileStorage, TrajectoryStorage};
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::main]
async fn main() -> SageResult<()> {
    println!("=== Trajectory Compression Demo ===\n");

    // Create a temporary directory for this demo
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a sample trajectory record
    let record = create_sample_trajectory();
    let record_id = record.id;

    println!("Sample trajectory ID: {}", record_id);
    println!("Task: {}\n", record.task);

    // Demo 1: Save without compression
    println!("--- Demo 1: Without Compression ---");
    let uncompressed_path = temp_path.join("uncompressed");
    let storage_uncompressed = FileStorage::with_compression(&uncompressed_path, false)?;

    storage_uncompressed.save(&record).await?;

    let stats_uncompressed = storage_uncompressed.statistics().await?;
    println!("Files created: {}", stats_uncompressed.total_records);
    println!("Total size: {} bytes", stats_uncompressed.total_size_bytes);
    println!();

    // Demo 2: Save with compression
    println!("--- Demo 2: With Compression ---");
    let compressed_path = temp_path.join("compressed");
    let storage_compressed = FileStorage::with_compression(&compressed_path, true)?;

    storage_compressed.save(&record).await?;

    let stats_compressed = storage_compressed.statistics().await?;
    println!("Files created: {}", stats_compressed.total_records);
    println!("Total size: {} bytes", stats_compressed.total_size_bytes);

    let compression_ratio =
        stats_uncompressed.total_size_bytes as f64 / stats_compressed.total_size_bytes as f64;
    println!("Compression ratio: {:.2}x", compression_ratio);
    println!(
        "Space saved: {} bytes ({:.1}%)",
        stats_uncompressed.total_size_bytes - stats_compressed.total_size_bytes,
        (1.0 - 1.0 / compression_ratio) * 100.0
    );
    println!();

    // Demo 3: Load compressed trajectory
    println!("--- Demo 3: Loading Compressed Trajectory ---");
    let loaded = storage_compressed.load(record_id).await?;

    match loaded {
        Some(trajectory) => {
            println!("Successfully loaded compressed trajectory!");
            println!("Task: {}", trajectory.task);
            println!("Success: {}", trajectory.success);
            println!("Execution time: {:.2}s", trajectory.execution_time);
            println!("Agent steps: {}", trajectory.agent_steps.len());
            println!("LLM interactions: {}", trajectory.llm_interactions.len());
        }
        None => println!("Failed to load trajectory"),
    }
    println!();

    // Demo 4: Configuration-based usage
    println!("--- Demo 4: Configuration Usage ---");
    println!("In sage_config.json, set:");
    println!(r#"  "trajectory": {{"#);
    println!(r#"    "directory": "trajectories","#);
    println!(r#"    "auto_save": true,"#);
    println!(r#"    "save_interval_steps": 5,"#);
    println!(r#"    "enable_compression": true"#);
    println!(r#"  }}"#);
    println!();
    println!("This will automatically compress all trajectory files during execution.");

    Ok(())
}

/// Create a sample trajectory for demonstration
fn create_sample_trajectory() -> TrajectoryRecord {
    TrajectoryRecord {
        id: Uuid::new_v4(),
        task: "Implement compression feature for trajectory files".to_string(),
        start_time: "2024-01-01T10:00:00Z".to_string(),
        end_time: "2024-01-01T10:15:00Z".to_string(),
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4".to_string(),
        max_steps: Some(20),
        llm_interactions: vec![LLMInteractionRecord {
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4".to_string(),
            input_messages: vec![serde_json::json!({
                "role": "user",
                "content": "Add gzip compression support for trajectory files"
            })],
            response: LLMResponseRecord {
                content: "I'll implement compression using the flate2 crate...".to_string(),
                model: Some("claude-sonnet-4".to_string()),
                finish_reason: Some("stop".to_string()),
                usage: Some(TokenUsageRecord {
                    input_tokens: 150,
                    output_tokens: 450,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                    reasoning_tokens: None,
                }),
                tool_calls: None,
            },
            tools_available: Some(vec![
                "str_replace_based_edit_tool".to_string(),
                "bash".to_string(),
            ]),
        }],
        agent_steps: vec![AgentStepRecord {
            step_number: 1,
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            state: "Running".to_string(),
            llm_messages: Some(vec![serde_json::json!({
                "role": "user",
                "content": "Add compression configuration"
            })]),
            llm_response: Some(LLMResponseRecord {
                content: "Adding compression configuration...".to_string(),
                model: Some("claude-sonnet-4".to_string()),
                finish_reason: Some("stop".to_string()),
                usage: Some(TokenUsageRecord {
                    input_tokens: 150,
                    output_tokens: 450,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                    reasoning_tokens: None,
                }),
                tool_calls: None,
            }),
            tool_calls: None,
            tool_results: None,
            reflection: Some("Implementation looks good".to_string()),
            error: None,
        }],
        success: true,
        final_result: Some("Compression feature successfully implemented".to_string()),
        execution_time: 900.0,
    }
}

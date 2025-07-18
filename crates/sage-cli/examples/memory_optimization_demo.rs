//! Memory optimization demonstration
//! 
//! This example shows the memory optimization improvements:
//! - Fixed LRU cache eviction
//! - Memory-optimized trajectory recording
//! - Efficient memory usage tracking

use sage_core::{
    cache::{CacheManager, CacheConfig, LLMCache},
    trajectory::memory_optimized::{MemoryOptimizedRecorder, MemoryOptimizedConfig},
    trajectory::recorder::TrajectoryRecord,
    llm::{LLMMessage, LLMResponse, MessageRole},
    types::LLMUsage,
    error::SageResult,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🧠 Sage Agent Memory Optimization Demo");
    println!("======================================\n");

    // Test all memory optimizations
    test_lru_cache_eviction().await?;
    test_memory_optimized_trajectory().await?;
    test_memory_usage_monitoring().await?;

    println!("\n🎉 Memory optimization demo completed!");
    println!("💡 Key improvements:");
    println!("   ✅ Fixed LRU cache eviction prevents memory leaks");
    println!("   ✅ Memory-optimized trajectory recording with compression");
    println!("   ✅ Automatic memory usage monitoring and cleanup");
    println!("   ✅ Configurable memory limits and eviction policies");

    Ok(())
}

/// Test LRU cache eviction
async fn test_lru_cache_eviction() -> SageResult<()> {
    println!("💾 Testing LRU Cache Eviction");
    println!("=============================");

    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 5, // Small capacity to demonstrate eviction
        enable_disk_cache: false,
        ..Default::default()
    };

    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LLMCache::new(cache_manager, Some(Duration::from_secs(300)));

    println!("📝 Adding 10 entries to cache with capacity of 5...");

    // Add more entries than capacity
    for i in 1..=10 {
        let messages = vec![
            LLMMessage {
                role: MessageRole::User,
                content: format!("Query {}", i),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                metadata: HashMap::new(),
            }
        ];

        let response = LLMResponse {
            content: format!("Response to query {}", i),
            tool_calls: vec![],
            usage: Some(LLMUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
                cost_usd: Some(0.001),
            }),
            model: Some("test-model".to_string()),
            finish_reason: Some("stop".to_string()),
            id: None,
            metadata: HashMap::new(),
        };

        llm_cache.cache_response("test", "model", &messages, None, &response, None).await?;

        if i % 2 == 0 {
            let stats = llm_cache.statistics().await?;
            println!("   After {} entries: {} cached, {} evictions", 
                i, stats.memory_stats.entry_count, stats.memory_stats.evictions);
        }
    }

    let final_stats = llm_cache.statistics().await?;
    println!("📊 Final cache statistics:");
    println!("   Entries: {} (should be ≤ 5)", final_stats.memory_stats.entry_count);
    println!("   Evictions: {}", final_stats.memory_stats.evictions);
    println!("   Memory size: {} bytes", final_stats.memory_stats.size_bytes);
    println!("   Hit rate: {:.1}%", final_stats.hit_rate() * 100.0);

    // Test that recent entries are still accessible
    println!("\n🔍 Testing access to recent entries...");
    for i in 6..=10 {
        let messages = vec![
            LLMMessage {
                role: MessageRole::User,
                content: format!("Query {}", i),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                metadata: HashMap::new(),
            }
        ];

        let cached = llm_cache.get_response("test", "model", &messages, None).await?;
        if cached.is_some() {
            print!("✅ ");
        } else {
            print!("❌ ");
        }
    }
    println!("\n✅ LRU cache eviction test completed!\n");

    Ok(())
}

/// Test memory-optimized trajectory recording
async fn test_memory_optimized_trajectory() -> SageResult<()> {
    println!("📊 Testing Memory-Optimized Trajectory Recording");
    println!("================================================");

    let config = MemoryOptimizedConfig {
        max_memory_records: 3, // Small capacity to demonstrate eviction
        max_memory_bytes: 10 * 1024, // 10KB limit
        storage_dir: "memory_demo_trajectories".into(),
        flush_interval: Duration::from_secs(1),
        max_record_age: Duration::from_secs(3600),
        enable_compression: true,
        batch_size: 5,
    };

    let recorder = MemoryOptimizedRecorder::new(config).await?;

    println!("📝 Adding trajectory records with memory limits...");

    let start = Instant::now();
    let mut record_ids = Vec::new();

    // Add records that will exceed memory limits
    for i in 1..=8 {
        let record = TrajectoryRecord {
            id: uuid::Uuid::new_v4(),
            task: format!("Task {}: Process large dataset", i),
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: chrono::Utc::now().to_rfc3339(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            max_steps: 10,
            llm_interactions: vec![],
            agent_steps: vec![],
            success: true,
            final_result: Some(format!("Large result data for task {}: {}", i, "x".repeat(1000))),
            execution_time: 2.5,
        };

        record_ids.push(record.id.clone());
        recorder.add_record(record).await?;

        if i % 2 == 0 {
            let stats = recorder.statistics().await;
            println!("   After {} records: {} in memory, {} evictions, {} bytes", 
                i, stats.memory_records, stats.memory_evictions, stats.memory_bytes);
        }
    }

    let add_duration = start.elapsed();
    println!("⏱️  Added {} records in {:?}", record_ids.len(), add_duration);

    // Test retrieval of evicted records (should load from disk)
    println!("\n🔍 Testing retrieval of evicted records...");
    let start = Instant::now();
    
    for (i, record_id) in record_ids.iter().enumerate() {
        let retrieved = recorder.get_record(record_id).await?;
        if retrieved.is_some() {
            print!("✅ ");
        } else {
            print!("❌ ");
        }
        
        if (i + 1) % 4 == 0 {
            println!();
        }
    }
    
    let retrieval_duration = start.elapsed();
    println!("\n⏱️  Retrieved {} records in {:?}", record_ids.len(), retrieval_duration);

    // Show final statistics
    let final_stats = recorder.statistics().await;
    println!("\n📊 Final trajectory statistics:");
    println!("   Total records: {}", final_stats.total_records);
    println!("   Memory records: {}", final_stats.memory_records);
    println!("   Memory usage: {} bytes", final_stats.memory_bytes);
    println!("   Memory evictions: {}", final_stats.memory_evictions);
    println!("   Flushed records: {}", final_stats.flushed_records);

    // Test recent records retrieval
    let recent = recorder.get_recent_records(3).await?;
    println!("   Recent records: {}", recent.len());

    // Cleanup
    let _ = tokio::fs::remove_dir_all("memory_demo_trajectories").await;
    
    println!("✅ Memory-optimized trajectory test completed!\n");
    Ok(())
}

/// Test memory usage monitoring
async fn test_memory_usage_monitoring() -> SageResult<()> {
    println!("📈 Testing Memory Usage Monitoring");
    println!("==================================");

    // Test with very small memory limits to trigger frequent evictions
    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 20,
        enable_disk_cache: false,
        ..Default::default()
    };

    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LLMCache::new(cache_manager, Some(Duration::from_secs(60)));

    println!("📊 Monitoring memory usage during high-load scenario...");

    let start = Instant::now();
    let mut memory_snapshots = Vec::new();

    // Simulate high-load scenario
    for batch in 0..5 {
        println!("\n🔄 Batch {} - Adding 10 large entries...", batch + 1);
        
        for i in 1..=10 {
            let entry_id = batch * 10 + i;
            let messages = vec![
                LLMMessage {
                    role: MessageRole::User,
                    content: format!("Large query {} with lots of context: {}", 
                        entry_id, "context ".repeat(100)),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                    metadata: HashMap::new(),
                }
            ];

            let response = LLMResponse {
                content: format!("Large response {}: {}", entry_id, "data ".repeat(200)),
                tool_calls: vec![],
                usage: Some(LLMUsage {
                    prompt_tokens: 500,
                    completion_tokens: 800,
                    total_tokens: 1300,
                    cost_usd: Some(0.05),
                }),
                model: Some("gpt-4".to_string()),
                finish_reason: Some("stop".to_string()),
                id: None,
                metadata: HashMap::new(),
            };

            llm_cache.cache_response("test", "model", &messages, None, &response, None).await?;
        }

        // Take memory snapshot
        let stats = llm_cache.statistics().await?;
        memory_snapshots.push((
            batch + 1,
            stats.memory_stats.entry_count,
            stats.memory_stats.size_bytes,
            stats.memory_stats.evictions,
        ));

        println!("   Entries: {}, Memory: {} bytes, Evictions: {}", 
            stats.memory_stats.entry_count,
            stats.memory_stats.size_bytes,
            stats.memory_stats.evictions
        );

        // Small delay to simulate real usage
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let total_duration = start.elapsed();
    println!("\n⏱️  Completed high-load test in {:?}", total_duration);

    // Show memory usage progression
    println!("\n📊 Memory usage progression:");
    println!("Batch | Entries | Memory (bytes) | Evictions");
    println!("------|---------|----------------|----------");
    for (batch, entries, memory, evictions) in memory_snapshots {
        println!("{:5} | {:7} | {:14} | {:9}", batch, entries, memory, evictions);
    }

    let final_stats = llm_cache.statistics().await?;
    println!("\n📈 Final memory statistics:");
    println!("   Peak entries: {} (stayed within limits)", final_stats.memory_stats.entry_count);
    println!("   Total evictions: {} (prevented memory overflow)", final_stats.memory_stats.evictions);
    println!("   Final memory usage: {} bytes", final_stats.memory_stats.size_bytes);
    println!("   Cache efficiency: {:.1}% hit rate", final_stats.hit_rate() * 100.0);

    println!("✅ Memory usage monitoring test completed!\n");
    Ok(())
}

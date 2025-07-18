//! Cache system demonstration
//! 
//! This example shows how to use the LLM response caching system
//! to improve performance and reduce API costs.

use sage_core::{
    cache::{CacheManager, CacheConfig, LLMCache},
    llm::{LLMMessage, MessageRole},
    error::SageResult,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Sage Agent Cache System Demo");
    println!("================================\n");

    // 1. Create cache configuration
    println!("📋 1. Setting up cache configuration");
    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 100,
        enable_disk_cache: true,
        disk_cache_dir: "cache_demo".to_string(),
        disk_capacity: 10 * 1024 * 1024, // 10MB
        default_ttl: Some(Duration::from_secs(3600)), // 1 hour
        llm_response_ttl: Some(Duration::from_secs(7200)), // 2 hours
        tool_result_ttl: Some(Duration::from_secs(1800)), // 30 minutes
        codebase_retrieval_ttl: Some(Duration::from_secs(3600)), // 1 hour
        cleanup_interval: Duration::from_secs(300), // 5 minutes
        max_entry_size: 1024 * 1024, // 1MB
    };
    println!("✅ Cache configured with memory and disk storage");

    // 2. Create cache manager
    println!("\n🔧 2. Creating cache manager");
    let cache_manager = CacheManager::new(cache_config)?;
    println!("✅ Cache manager created");

    // 3. Create LLM cache
    println!("\n💾 3. Setting up LLM cache");
    let llm_cache = LLMCache::new(
        cache_manager,
        Some(Duration::from_secs(3600)) // 1 hour default TTL
    );
    println!("✅ LLM cache ready");

    // 4. Simulate LLM requests and caching
    println!("\n🤖 4. Simulating LLM requests");
    
    let messages = vec![
        LLMMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant.".to_string(),
        },
        LLMMessage {
            role: MessageRole::User,
            content: "What is the capital of France?".to_string(),
        },
    ];

    let provider = "openai";
    let model = "gpt-4";

    // Check if response is cached
    println!("🔍 Checking cache for request...");
    let cached_response = llm_cache.get_response(provider, model, &messages, None).await?;
    
    if cached_response.is_some() {
        println!("✅ Found cached response!");
    } else {
        println!("❌ No cached response found");
        
        // Simulate an LLM response (normally this would come from actual API call)
        println!("📡 Simulating API call...");
        let simulated_response = sage_core::llm::LLMResponse {
            content: "The capital of France is Paris.".to_string(),
            tool_calls: Vec::new(),
            usage: sage_core::types::LLMUsage {
                prompt_tokens: 25,
                completion_tokens: 8,
                total_tokens: 33,
            },
            model: model.to_string(),
            finish_reason: Some("stop".to_string()),
        };
        
        // Cache the response
        println!("💾 Caching response...");
        llm_cache.cache_response(
            provider,
            model,
            &messages,
            None,
            &simulated_response,
            None, // Use default TTL
        ).await?;
        println!("✅ Response cached successfully");
    }

    // 5. Test cache hit
    println!("\n🎯 5. Testing cache hit");
    let cached_response = llm_cache.get_response(provider, model, &messages, None).await?;
    
    if let Some(response) = cached_response {
        println!("✅ Cache hit! Response: {}", response.content);
        println!("📊 Tokens used: {}", response.usage.total_tokens);
    } else {
        println!("❌ Unexpected cache miss");
    }

    // 6. Cache statistics
    println!("\n📈 6. Cache statistics");
    let stats = llm_cache.statistics().await?;
    println!("Memory cache:");
    println!("  - Entries: {}", stats.memory_stats.entry_count);
    println!("  - Size: {} bytes", stats.memory_stats.size_bytes);
    println!("  - Hits: {}", stats.memory_stats.hits);
    println!("  - Misses: {}", stats.memory_stats.misses);
    
    if let Some(disk_stats) = &stats.disk_stats {
        println!("Disk cache:");
        println!("  - Entries: {}", disk_stats.entry_count);
        println!("  - Size: {} bytes", disk_stats.size_bytes);
        println!("  - Hits: {}", disk_stats.hits);
        println!("  - Misses: {}", disk_stats.misses);
    }
    
    println!("Overall:");
    println!("  - Total hits: {}", stats.total_hits);
    println!("  - Total misses: {}", stats.total_misses);
    println!("  - Hit rate: {:.2}%", stats.hit_rate() * 100.0);

    // 7. Test different requests
    println!("\n🔄 7. Testing different requests");
    let different_messages = vec![
        LLMMessage {
            role: MessageRole::User,
            content: "What is 2 + 2?".to_string(),
        },
    ];

    let is_cached = llm_cache.is_cached(provider, model, &different_messages, None).await?;
    println!("Different request cached: {}", is_cached);

    // 8. Cleanup demonstration
    println!("\n🧹 8. Cache cleanup");
    println!("Cleaning up expired entries...");
    llm_cache.cleanup_expired().await?;
    println!("✅ Cleanup completed");

    // Final statistics
    let final_stats = llm_cache.statistics().await?;
    println!("\n📊 Final statistics:");
    println!("Total entries: {}", final_stats.total_entries());
    println!("Total size: {} bytes", final_stats.total_size_bytes());
    println!("Hit rate: {:.2}%", final_stats.hit_rate() * 100.0);

    println!("\n🎉 Cache demo completed successfully!");
    println!("💡 The cache system can significantly reduce API costs and improve response times.");

    Ok(())
}

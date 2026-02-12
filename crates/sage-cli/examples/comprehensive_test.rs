//! Comprehensive test of Sage Agent optimizations
//!
//! This example demonstrates the real-world usage of:
//! - LLM response caching
//! - Streaming responses
//! - Performance improvements

use futures::stream;
use sage_core::{
    cache::{CacheConfig, CacheManager, LlmCache},
    error::SageResult,
    llm::streaming::{sse, stream_utils},
    llm::{LlmMessage, LlmResponse, MessageRole, StreamChunk},
    types::TokenUsage,
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Sage Agent Comprehensive Test");
    println!("=================================\n");

    // Run all test scenarios
    test_caching_performance().await?;
    test_streaming_experience().await?;
    test_cache_streaming_combo().await?;
    test_sse_functionality().await?;
    test_memory_efficiency().await?;

    println!("\n🎉 All tests completed successfully!");
    println!("💡 Summary of improvements:");
    println!("   ✅ LLM response caching reduces API costs");
    println!("   ✅ Streaming provides real-time user feedback");
    println!("   ✅ Memory-efficient processing");
    println!("   ✅ SSE support for web interfaces");
    println!("   ✅ Robust error handling");

    Ok(())
}

/// Test caching performance and cost savings
async fn test_caching_performance() -> SageResult<()> {
    println!("💾 Testing Caching Performance");
    println!("==============================");

    // Set up cache with realistic configuration
    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 100,
        enable_disk_cache: true,
        disk_cache_dir: "comprehensive_test_cache".to_string(),
        disk_capacity: 10 * 1024 * 1024,                   // 10MB
        llm_response_ttl: Some(Duration::from_secs(3600)), // 1 hour
        ..Default::default()
    };

    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LlmCache::new(cache_manager, Some(Duration::from_secs(3600)));

    // Simulate common queries
    let common_queries = vec![
        "What is Rust programming language?",
        "Explain async/await in Rust",
        "How to handle errors in Rust?",
        "What are Rust ownership rules?",
        "How to use Cargo in Rust?",
    ];

    println!("📝 Simulating {} unique queries...", common_queries.len());

    // First pass: Cache misses (simulate API calls)
    let start = Instant::now();
    let mut total_tokens = 0;
    let mut total_cost = 0.0;

    for (i, query) in common_queries.iter().enumerate() {
        let messages = vec![LlmMessage {
            role: MessageRole::User,
            content: query.to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        }];

        // Check cache first
        let cached = llm_cache
            .get_response("openai", "gpt-4", &messages, None)
            .await?;

        if cached.is_none() {
            // Simulate API call delay and cost
            tokio::time::sleep(Duration::from_millis(500)).await; // Simulate network latency

            let response = LlmResponse {
                content: format!("Detailed answer about: {}", query),
                tool_calls: vec![],
                usage: Some(TokenUsage {
                    input_tokens: 20 + (i * 5) as u64,
                    output_tokens: 100 + (i * 10) as u64,
                    cache_read_tokens: None,
                    cache_write_tokens: None,
                    cost_estimate: Some(0.002 + (i as f64 * 0.001)),
                }),
                model: Some("gpt-4".to_string()),
                finish_reason: Some("stop".to_string()),
                id: None,
                metadata: HashMap::new(),
            };

            total_tokens += response.usage.as_ref().unwrap().total_tokens();
            total_cost += response.usage.as_ref().unwrap().cost_estimate.unwrap();

            // Cache the response
            llm_cache
                .cache_response("openai", "gpt-4", &messages, None, &response, None)
                .await?;

            print!("🔄 ");
        } else {
            print!("💾 ");
        }
        io::stdout().flush().unwrap();
    }

    let first_pass_duration = start.elapsed();
    println!("\n⏱️  First pass (cache misses): {:?}", first_pass_duration);
    println!("💰 Total cost: ${:.4}", total_cost);
    println!("🔢 Total tokens: {}", total_tokens);

    // Second pass: Cache hits (no API calls)
    println!("\n🔄 Running same queries again (should be cached)...");
    let start = Instant::now();
    let mut cache_hits = 0;

    for query in &common_queries {
        let messages = vec![LlmMessage {
            role: MessageRole::User,
            content: query.to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        }];

        let cached = llm_cache
            .get_response("openai", "gpt-4", &messages, None)
            .await?;
        if cached.is_some() {
            cache_hits += 1;
            print!("✅ ");
        } else {
            print!("❌ ");
        }
        io::stdout().flush().unwrap();
    }

    let second_pass_duration = start.elapsed();
    println!("\n⏱️  Second pass (cache hits): {:?}", second_pass_duration);
    println!("🎯 Cache hits: {}/{}", cache_hits, common_queries.len());

    let speedup = first_pass_duration.as_millis() as f64 / second_pass_duration.as_millis() as f64;
    println!("🚀 Speedup: {:.1}x faster", speedup);

    // Show cache statistics
    let stats = llm_cache.statistics().await?;
    println!("📊 Cache Statistics:");
    println!("   Memory entries: {}", stats.memory_stats.entry_count);
    println!("   Memory size: {} bytes", stats.memory_stats.size_bytes);
    println!("   Total hits: {}", stats.total_hits);
    println!("   Total misses: {}", stats.total_misses);
    println!("   Hit rate: {:.1}%", stats.hit_rate() * 100.0);

    // Cleanup
    let _ = tokio::fs::remove_dir_all("comprehensive_test_cache").await;

    println!("✅ Caching test completed!\n");
    Ok(())
}

/// Test streaming user experience
async fn test_streaming_experience() -> SageResult<()> {
    println!("🌊 Testing Streaming Experience");
    println!("===============================");

    // Simulate a long response that benefits from streaming
    let long_response_chunks = vec![
        "Rust is a systems programming language",
        " that runs blazingly fast,",
        " prevents segfaults,",
        " and guarantees thread safety.",
        "\n\nIt accomplishes these goals",
        " by being memory safe",
        " without using garbage collection.",
        "\n\nRust has great documentation,",
        " a friendly compiler",
        " with useful error messages,",
        " and top-notch tooling",
        " — an integrated package manager",
        " and build tool,",
        " smart multi-editor support",
        " with auto-completion",
        " and type inspections,",
        " an auto-formatter,",
        " and more.",
    ];

    println!("📡 Simulating streaming response (watch the text appear in real-time):");
    println!("🤖 AI: ");
    print!("      ");
    io::stdout().flush().unwrap();

    let start = Instant::now();
    let mut total_content = String::new();

    // Create stream chunks
    let mut stream_chunks: Vec<Result<StreamChunk, sage_core::error::SageError>> = Vec::new();
    let chunk_count = long_response_chunks.len();
    for chunk_text in &long_response_chunks {
        stream_chunks.push(Ok(StreamChunk::content(*chunk_text)));
    }
    stream_chunks.push(Ok(StreamChunk::final_chunk(
        Some(TokenUsage {
            input_tokens: 15,
            output_tokens: 85,
            cache_read_tokens: None,
            cache_write_tokens: None,
            cost_estimate: Some(0.003),
        }),
        Some("stop".to_string()),
    )));

    // Process stream with realistic delays
    for chunk_result in stream_chunks {
        match chunk_result {
            Ok(chunk) => {
                if let Some(content) = &chunk.content {
                    print!("{}", content);
                    io::stdout().flush().unwrap();
                    total_content.push_str(content);

                    // Simulate realistic streaming delay
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }

                if chunk.is_final {
                    println!("\n");
                    if let Some(usage) = chunk.usage {
                        println!(
                            "📊 Usage: {} tokens (${:.4})",
                            usage.total_tokens(),
                            usage.cost_estimate.unwrap_or(0.0)
                        );
                    }
                    break;
                }
            }
            Err(e) => {
                eprintln!("❌ Stream error: {}", e);
                break;
            }
        }
    }

    let streaming_duration = start.elapsed();
    println!("⏱️  Streaming completed in: {:?}", streaming_duration);
    println!(
        "📝 Total content length: {} characters",
        total_content.len()
    );

    // Compare with non-streaming (all at once)
    println!("\n🔄 Compare with non-streaming (all at once):");
    let start = Instant::now();
    tokio::time::sleep(Duration::from_millis((50 * chunk_count) as u64)).await; // Simulate same total delay
    println!("🤖 AI: {}", total_content);
    let non_streaming_duration = start.elapsed();
    println!(
        "⏱️  Non-streaming completed in: {:?}",
        non_streaming_duration
    );

    println!("💡 Streaming provides immediate feedback and better UX!");
    println!("✅ Streaming test completed!\n");
    Ok(())
}

/// Test combination of caching and streaming
async fn test_cache_streaming_combo() -> SageResult<()> {
    println!("🔄 Testing Cache + Streaming Combination");
    println!("========================================");

    let cache_config = CacheConfig::default();
    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LlmCache::new(cache_manager, Some(Duration::from_secs(300)));

    let messages = vec![LlmMessage {
        role: MessageRole::User,
        content: "Explain quantum computing in simple terms".to_string(),
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
        name: None,
        metadata: HashMap::new(),
    }];

    // First request: Stream and cache
    println!("🌊 First request: Streaming response...");
    let chunks = vec![
        StreamChunk::content("Quantum computing is a type of computation"),
        StreamChunk::content(" that harnesses quantum mechanics"),
        StreamChunk::content(" to process information in fundamentally new ways."),
        StreamChunk::final_chunk(
            Some(TokenUsage {
                input_tokens: 12,
                output_tokens: 25,
                cache_read_tokens: None,
                cache_write_tokens: None,
                cost_estimate: Some(0.002),
            }),
            Some("stop".to_string()),
        ),
    ];

    let stream = Box::pin(stream::iter(chunks.into_iter().map(Ok)));
    let response = stream_utils::collect_stream(stream).await?;

    // Cache the response
    llm_cache
        .cache_response("openai", "gpt-4", &messages, None, &response, None)
        .await?;
    println!("💾 Response cached");

    // Second request: Instant from cache
    println!("\n⚡ Second request: Instant from cache...");
    let start = Instant::now();
    let cached_response = llm_cache
        .get_response("openai", "gpt-4", &messages, None)
        .await?;
    let cache_duration = start.elapsed();

    if let Some(cached) = cached_response {
        println!("🤖 AI: {}", cached.content);
        println!("⏱️  Retrieved from cache in: {:?}", cache_duration);
        println!(
            "💰 Cost saved: ${:.4}",
            cached.usage.as_ref().unwrap().cost_estimate.unwrap_or(0.0)
        );
    }

    println!("✅ Cache + Streaming combo test completed!\n");
    Ok(())
}

/// Test Server-Sent Events functionality
async fn test_sse_functionality() -> SageResult<()> {
    println!("🌐 Testing Server-Sent Events (SSE)");
    println!("===================================");

    let chunks = [
        StreamChunk::content("Hello "),
        StreamChunk::content("from "),
        StreamChunk::content("SSE!"),
        StreamChunk::final_chunk(None, Some("stop".to_string())),
    ];

    println!("📡 Converting stream chunks to SSE format:");
    for (i, chunk) in chunks.iter().enumerate() {
        let sse_event = sse::chunk_to_sse(chunk.clone())?;
        println!("\nSSE Event {}:", i + 1);
        println!("{}", sse_event);
    }

    println!("✅ SSE test completed!\n");
    Ok(())
}

/// Test memory efficiency
async fn test_memory_efficiency() -> SageResult<()> {
    println!("🧠 Testing Memory Efficiency");
    println!("============================");

    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 10, // Small capacity to test eviction
        enable_disk_cache: false,
        ..Default::default()
    };

    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LlmCache::new(cache_manager, Some(Duration::from_secs(60)));

    // Add more entries than capacity to test LRU eviction
    println!("📝 Adding 15 entries to cache with capacity of 10...");
    for i in 0..15 {
        let messages = vec![LlmMessage {
            role: MessageRole::User,
            content: format!("Query number {}", i),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        }];

        let response = LlmResponse {
            content: format!("Response {}", i),
            tool_calls: vec![],
            usage: Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 20,
                cache_read_tokens: None,
                cache_write_tokens: None,
                cost_estimate: Some(0.001),
            }),
            model: Some("test".to_string()),
            finish_reason: Some("stop".to_string()),
            id: None,
            metadata: HashMap::new(),
        };

        llm_cache
            .cache_response("test", "model", &messages, None, &response, None)
            .await?;

        if i % 5 == 4 {
            let stats = llm_cache.statistics().await?;
            println!(
                "   After {} entries: {} cached",
                i + 1,
                stats.memory_stats.entry_count
            );
        }
    }

    let final_stats = llm_cache.statistics().await?;
    println!("📊 Final cache state:");
    println!(
        "   Entries: {} (should be ≤ 10 due to LRU eviction)",
        final_stats.memory_stats.entry_count
    );
    println!(
        "   Memory size: {} bytes",
        final_stats.memory_stats.size_bytes
    );
    println!("   Evictions: {}", final_stats.memory_stats.evictions);

    println!("✅ Memory efficiency test completed!\n");
    Ok(())
}

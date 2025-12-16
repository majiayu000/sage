//! Integration tests for Sage Agent core functionality
//!
//! This module tests the interaction between different components
//! like caching, streaming, and LLM clients.

use futures::{StreamExt, stream};
use sage_core::{
    cache::{CacheConfig, CacheManager, LLMCache},
    error::SageResult,
    llm::streaming::stream_utils,
    llm::{LLMMessage, LLMResponse, MessageRole, StreamChunk},
    types::LLMUsage,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::fs;

/// Test cache and streaming integration
#[tokio::test]
async fn test_cache_streaming_integration() -> SageResult<()> {
    println!("🧪 Testing cache and streaming integration");

    // 1. Set up cache
    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 50,
        enable_disk_cache: true,
        disk_cache_dir: "test_cache_integration".to_string(),
        disk_capacity: 1024 * 1024,                  // 1MB
        default_ttl: Some(Duration::from_secs(300)), // 5 minutes
        ..Default::default()
    };

    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LLMCache::new(cache_manager, Some(Duration::from_secs(300)));

    // 2. Create test messages
    let messages = vec![
        LLMMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant.".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        },
        LLMMessage {
            role: MessageRole::User,
            content: "Tell me about Rust programming.".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        },
    ];

    let provider = "openai";
    let model = "gpt-4";

    // 3. Test cache miss (first request)
    println!("🔍 Testing cache miss...");
    let cached_response = llm_cache
        .get_response(provider, model, &messages, None)
        .await?;
    assert!(
        cached_response.is_none(),
        "Should be cache miss on first request"
    );

    // 4. Simulate streaming response and cache it
    println!("🌊 Simulating streaming response...");
    let stream_chunks = vec![
        StreamChunk::content("Rust is a systems programming language"),
        StreamChunk::content(" that focuses on safety, speed, and concurrency."),
        StreamChunk::content(" It was originally developed by Mozilla"),
        StreamChunk::content(" and has gained widespread adoption."),
        StreamChunk::final_chunk(
            Some(LLMUsage {
                prompt_tokens: 25,
                completion_tokens: 20,
                total_tokens: 45,
                cost_usd: Some(0.002),
            }),
            Some("stop".to_string()),
        ),
    ];

    // Convert chunks to stream and collect
    let mock_stream = Box::pin(stream::iter(stream_chunks.into_iter().map(Ok)));
    let complete_response = stream_utils::collect_stream(mock_stream).await?;

    println!("✅ Collected response: {}", complete_response.content);
    assert!(!complete_response.content.is_empty());
    assert!(complete_response.usage.is_some());

    // 5. Cache the response
    println!("💾 Caching response...");
    llm_cache
        .cache_response(provider, model, &messages, None, &complete_response, None)
        .await?;

    // 6. Test cache hit (second request)
    println!("🎯 Testing cache hit...");
    let cached_response = llm_cache
        .get_response(provider, model, &messages, None)
        .await?;
    assert!(
        cached_response.is_some(),
        "Should be cache hit on second request"
    );

    let cached = cached_response.unwrap();
    assert_eq!(cached.content, complete_response.content);
    assert_eq!(cached.usage.as_ref().unwrap().total_tokens, 45);

    // 7. Test cache statistics
    println!("📊 Checking cache statistics...");
    let stats = llm_cache.statistics().await?;
    assert!(stats.total_hits > 0, "Should have cache hits");
    assert!(
        stats.memory_stats.entry_count > 0,
        "Should have cached entries"
    );
    println!("Cache hit rate: {:.2}%", stats.hit_rate() * 100.0);

    // 8. Test different request (should be cache miss)
    let different_messages = vec![LLMMessage {
        role: MessageRole::User,
        content: "What is Python?".to_string(),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        metadata: HashMap::new(),
    }];

    let different_cached = llm_cache
        .get_response(provider, model, &different_messages, None)
        .await?;
    assert!(
        different_cached.is_none(),
        "Different request should be cache miss"
    );

    // 9. Cleanup
    let _ = fs::remove_dir_all("test_cache_integration").await;

    println!("✅ Cache and streaming integration test passed!");
    Ok(())
}

/// Test stream utilities
#[tokio::test]
async fn test_stream_utilities() -> SageResult<()> {
    println!("🧪 Testing stream utilities");

    // 1. Test content filtering
    println!("🔍 Testing content filtering...");
    let mixed_chunks = vec![
        Ok(StreamChunk::content("Hello ")),
        Ok(StreamChunk::tool_calls(vec![])), // Should be filtered out
        Ok(StreamChunk::content("world!")),
        Ok(StreamChunk::final_chunk(None, Some("stop".to_string()))),
    ];

    let mixed_stream = Box::pin(stream::iter(mixed_chunks));
    let content_stream = stream_utils::content_only(mixed_stream);

    let mut content_parts = Vec::new();
    let mut content_stream = content_stream;
    while let Some(chunk_result) = content_stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(content) = chunk.content {
                    content_parts.push(content);
                }
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    assert_eq!(content_parts.len(), 2);
    assert_eq!(content_parts[0], "Hello ");
    assert_eq!(content_parts[1], "world!");

    // 2. Test stream mapping
    println!("🔄 Testing stream mapping...");
    let chunks = vec![
        Ok(StreamChunk::content("hello")),
        Ok(StreamChunk::content("world")),
    ];

    let stream = Box::pin(stream::iter(chunks));
    let mapped_stream = stream_utils::map_stream(stream, |mut chunk| {
        if let Some(content) = &chunk.content {
            chunk.content = Some(content.to_uppercase());
        }
        chunk
    });

    let mut mapped_stream = mapped_stream;
    let mut results = Vec::new();
    while let Some(chunk_result) = mapped_stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(content) = chunk.content {
                    results.push(content);
                }
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    assert_eq!(results, vec!["HELLO", "WORLD"]);

    // 3. Test stream collection
    println!("📦 Testing stream collection...");
    let chunks = vec![
        Ok(StreamChunk::content("Part 1 ")),
        Ok(StreamChunk::content("Part 2 ")),
        Ok(StreamChunk::content("Part 3")),
        Ok(StreamChunk::final_chunk(
            Some(LLMUsage {
                prompt_tokens: 10,
                completion_tokens: 15,
                total_tokens: 25,
                cost_usd: Some(0.001),
            }),
            Some("stop".to_string()),
        )),
    ];

    let stream = Box::pin(stream::iter(chunks));
    let response = stream_utils::collect_stream(stream).await?;

    assert_eq!(response.content, "Part 1 Part 2 Part 3");
    assert_eq!(response.usage.unwrap().total_tokens, 25);
    assert_eq!(response.finish_reason, Some("stop".to_string()));

    println!("✅ Stream utilities test passed!");
    Ok(())
}

/// Test cache performance under load
#[tokio::test]
async fn test_cache_performance() -> SageResult<()> {
    println!("🧪 Testing cache performance");

    let cache_config = CacheConfig {
        enable_memory_cache: true,
        memory_capacity: 100,
        enable_disk_cache: false, // Disable disk for performance test
        ..Default::default()
    };

    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LLMCache::new(cache_manager, Some(Duration::from_secs(60)));

    // Generate test data
    let mut test_cases = Vec::new();
    for i in 0..50 {
        let messages = vec![LLMMessage {
            role: MessageRole::User,
            content: format!("Test message {}", i),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }];

        let response = LLMResponse {
            content: format!("Response to test message {}", i),
            tool_calls: vec![],
            usage: Some(LLMUsage {
                prompt_tokens: 10,
                completion_tokens: 15,
                total_tokens: 25,
                cost_usd: Some(0.001),
            }),
            model: Some("test-model".to_string()),
            finish_reason: Some("stop".to_string()),
            id: None,
            metadata: HashMap::new(),
        };

        test_cases.push((messages, response));
    }

    // Measure cache write performance
    let start = std::time::Instant::now();
    for (messages, response) in &test_cases {
        llm_cache
            .cache_response("test", "model", messages, None, response, None)
            .await?;
    }
    let write_duration = start.elapsed();
    println!(
        "📝 Cached {} entries in {:?}",
        test_cases.len(),
        write_duration
    );

    // Measure cache read performance
    let start = std::time::Instant::now();
    let mut hits = 0;
    for (messages, _) in &test_cases {
        if llm_cache
            .get_response("test", "model", messages, None)
            .await?
            .is_some()
        {
            hits += 1;
        }
    }
    let read_duration = start.elapsed();
    println!(
        "🔍 Read {} entries ({} hits) in {:?}",
        test_cases.len(),
        hits,
        read_duration
    );

    assert_eq!(hits, test_cases.len(), "All entries should be cache hits");

    // Check final statistics
    let stats = llm_cache.statistics().await?;
    println!(
        "📊 Final stats: {} entries, {:.2}% hit rate",
        stats.memory_stats.entry_count,
        stats.hit_rate() * 100.0
    );

    println!("✅ Cache performance test passed!");
    Ok(())
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() -> SageResult<()> {
    println!("🧪 Testing error handling");

    // Test invalid cache directory
    let invalid_config = CacheConfig {
        enable_disk_cache: true,
        disk_cache_dir: "/invalid/path/that/should/not/exist".to_string(),
        ..Default::default()
    };

    // This should handle the error gracefully
    match CacheManager::new(invalid_config) {
        Ok(_) => println!("⚠️  Cache manager created despite invalid path"),
        Err(e) => println!("✅ Expected error for invalid cache path: {}", e),
    }

    // Test stream error handling
    let error_chunks = vec![
        Ok(StreamChunk::content("Good chunk")),
        Err(sage_core::error::SageError::llm(
            "Simulated stream error".to_string(),
        )),
        Ok(StreamChunk::content("Another good chunk")),
    ];

    let error_stream = Box::pin(stream::iter(error_chunks));
    let mut error_count = 0;
    let mut success_count = 0;

    let mut error_stream = error_stream;
    while let Some(chunk_result) = error_stream.next().await {
        match chunk_result {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    assert_eq!(success_count, 2);
    assert_eq!(error_count, 1);

    println!("✅ Error handling test passed!");
    Ok(())
}

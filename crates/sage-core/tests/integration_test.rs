//! Integration tests for Sage Agent core functionality
//!
//! This module tests the interaction between different components
//! like caching, streaming, and LLM clients.

use futures::{StreamExt, stream};
use sage_core::{
    cache::{CacheConfig, CacheManager, LlmCache},
    error::SageResult,
    llm::streaming::stream_utils,
    llm::{LlmMessage, LlmResponse, MessageRole, StreamChunk},
    types::LlmUsage,
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
    let llm_cache = LlmCache::new(cache_manager, Some(Duration::from_secs(300)));

    // 2. Create test messages
    let messages = vec![
        LlmMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant.".to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        },
        LlmMessage {
            role: MessageRole::User,
            content: "Tell me about Rust programming.".to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
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
            Some(LlmUsage {
                prompt_tokens: 25,
                completion_tokens: 20,
                total_tokens: 45,
                cost_usd: Some(0.002),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
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
    let different_messages = vec![LlmMessage {
        role: MessageRole::User,
        content: "What is Python?".to_string(),
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
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
            Some(LlmUsage {
                prompt_tokens: 10,
                completion_tokens: 15,
                total_tokens: 25,
                cost_usd: Some(0.001),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
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
    let llm_cache = LlmCache::new(cache_manager, Some(Duration::from_secs(60)));

    // Generate test data
    let mut test_cases = Vec::new();
    for i in 0..50 {
        let messages = vec![LlmMessage {
            role: MessageRole::User,
            content: format!("Test message {}", i),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        }];

        let response = LlmResponse {
            content: format!("Response to test message {}", i),
            tool_calls: vec![],
            usage: Some(LlmUsage {
                prompt_tokens: 10,
                completion_tokens: 15,
                total_tokens: 25,
                cost_usd: Some(0.001),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
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

/// Test auto-compact with boundary system (Claude Code style)
#[tokio::test]
async fn test_auto_compact_boundary_system() -> SageResult<()> {
    use sage_core::context::{
        is_compact_boundary, slice_from_last_compact_boundary, AutoCompact, AutoCompactConfig,
    };

    println!("🧪 Testing auto-compact boundary system");

    // 1. Create config with low threshold for easier testing
    // Using reserved_for_response: 1000 tokens reserved, threshold = 2000 - 1000 = 1000 tokens
    let config = AutoCompactConfig {
        max_context_tokens: 2000,
        reserved_for_response: 1000, // Threshold at 1000 tokens (50%)
        min_messages_to_keep: 3,
        preserve_recent_count: 2,
        preserve_system_messages: true,
        preserve_tool_messages: false,
        ..Default::default()
    };

    let mut auto_compact = AutoCompact::new(config);

    // 2. Create messages that exceed threshold
    println!("📝 Creating test messages...");
    let mut messages: Vec<LlmMessage> = vec![
        LlmMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant.".to_string(),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        },
    ];

    // Add enough messages to trigger compact
    for i in 0..20 {
        messages.push(LlmMessage {
            role: if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            },
            content: format!(
                "Message {} with some content to fill the context window quickly",
                i
            ),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        });
    }

    let original_count = messages.len();
    println!("📊 Original message count: {}", original_count);

    // 3. Force compact
    println!("🗜️ Forcing compact...");
    let result = auto_compact.force_compact(&mut messages).await?;

    assert!(result.was_compacted, "Should have compacted");
    assert!(result.compact_id.is_some(), "Should have compact ID");
    assert!(
        result.messages_after < result.messages_before,
        "Should have fewer messages after compact"
    );
    println!(
        "✅ Compacted: {} -> {} messages (saved {} tokens)",
        result.messages_before,
        result.messages_after,
        result.tokens_saved()
    );

    // 4. Verify boundary marker exists
    println!("🔍 Checking boundary marker...");
    let has_boundary = messages.iter().any(|m| is_compact_boundary(m));
    assert!(has_boundary, "Should have a compact boundary marker");
    println!("✅ Boundary marker found");

    // 5. Verify slice_from_last_compact_boundary works
    let sliced = slice_from_last_compact_boundary(&messages);
    assert!(
        sliced.len() <= messages.len(),
        "Sliced messages should be <= total"
    );
    assert!(
        is_compact_boundary(&sliced[0]),
        "First message in slice should be boundary"
    );
    println!(
        "✅ Slice from boundary: {} messages (boundary + summary + kept)",
        sliced.len()
    );

    // 6. Add more messages after compact
    println!("📝 Adding messages after compact...");
    for i in 0..5 {
        messages.push(LlmMessage {
            role: MessageRole::User,
            content: format!("New message {} after compact", i),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        });
    }

    // 7. Check if needs compaction (should only consider post-boundary messages)
    let needs_compact = auto_compact.needs_compaction(&messages);
    println!(
        "📊 Needs compaction after adding 5 messages: {}",
        needs_compact
    );
    // With only 5 new messages, shouldn't need compact yet
    assert!(
        !needs_compact,
        "Should not need compaction with few new messages"
    );

    // 8. Test second compact creates new boundary
    println!("📝 Adding more messages to trigger second compact...");
    for i in 0..30 {
        messages.push(LlmMessage {
            role: MessageRole::User,
            content: format!("Bulk message {} to trigger second compact", i),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        });
    }

    let result2 = auto_compact.force_compact(&mut messages).await?;
    assert!(result2.was_compacted, "Second compact should work");
    assert_ne!(
        result.compact_id, result2.compact_id,
        "Should have different compact IDs"
    );
    println!(
        "✅ Second compact: {} -> {} messages (ID: {:?})",
        result2.messages_before,
        result2.messages_after,
        result2.compact_id.map(|id| id.to_string()[..8].to_string())
    );

    // 9. Verify stats
    let stats = auto_compact.stats();
    assert_eq!(stats.total_compactions, 2, "Should have 2 compactions");
    assert!(stats.total_tokens_saved > 0, "Should have saved tokens");
    println!(
        "📊 Stats: {} compactions, {} tokens saved",
        stats.total_compactions, stats.total_tokens_saved
    );

    println!("✅ Auto-compact boundary system test passed!");
    Ok(())
}

/// Test compact with custom instructions
#[tokio::test]
async fn test_compact_with_custom_instructions() -> SageResult<()> {
    use sage_core::context::{AutoCompact, AutoCompactConfig};

    println!("🧪 Testing compact with custom instructions");

    // reserved_for_response: 350 tokens, threshold = 500 - 350 = 150 tokens (30%)
    let config = AutoCompactConfig {
        max_context_tokens: 500,
        reserved_for_response: 350,
        ..Default::default()
    };

    let mut auto_compact = AutoCompact::new(config);

    // Create messages
    let mut messages: Vec<LlmMessage> = (0..15)
        .map(|i| LlmMessage {
            role: if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            },
            content: format!("Test message {} with TypeScript code: const x = {}", i, i),
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            name: None,
            metadata: HashMap::new(),
        })
        .collect();

    // Compact with custom instructions
    let result = auto_compact
        .compact_with_instructions(&mut messages, "Focus on TypeScript code changes")
        .await?;

    assert!(result.was_compacted, "Should have compacted");
    assert!(
        result.summary_preview.is_some(),
        "Should have summary preview"
    );

    // The summary should include our conversation (simple summary without LLM)
    let preview = result.summary_preview.unwrap();
    assert!(!preview.is_empty(), "Summary preview should not be empty");
    println!("✅ Summary preview: {}...", &preview[..preview.len().min(100)]);

    println!("✅ Custom instructions test passed!");
    Ok(())
}

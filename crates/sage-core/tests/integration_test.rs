//! Integration tests for Sage Agent core functionality
//!
//! Tests the interaction between streaming utilities and context management.

use futures::{StreamExt, stream};
use sage_core::{
    error::SageResult,
    llm::streaming::stream_utils,
    llm::{LlmMessage, LlmResponse, MessageRole, StreamChunk},
    types::TokenUsage,
};
use std::collections::HashMap;

/// Test stream utilities
#[tokio::test]
async fn test_stream_utilities() -> SageResult<()> {
    // 1. Test content filtering
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
    let chunks = vec![
        Ok(StreamChunk::content("Part 1 ")),
        Ok(StreamChunk::content("Part 2 ")),
        Ok(StreamChunk::content("Part 3")),
        Ok(StreamChunk::final_chunk(
            Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 15,
                cache_read_tokens: None,
                cache_write_tokens: None,
                cost_estimate: Some(0.001),
            }),
            Some("stop".to_string()),
        )),
    ];

    let stream = Box::pin(stream::iter(chunks));
    let response = stream_utils::collect_stream(stream).await?;

    assert_eq!(response.content, "Part 1 Part 2 Part 3");
    assert_eq!(response.usage.unwrap().total_tokens(), 25);
    assert_eq!(response.finish_reason, Some("stop".to_string()));

    Ok(())
}

/// Test error handling in streams
#[tokio::test]
async fn test_error_handling() -> SageResult<()> {
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

    Ok(())
}

/// Test auto-compact with boundary system
#[tokio::test]
async fn test_auto_compact_boundary_system() -> SageResult<()> {
    use sage_core::context::{
        AutoCompact, AutoCompactConfig, is_compact_boundary, slice_from_last_compact_boundary,
    };

    let config = AutoCompactConfig {
        max_context_tokens: 2000,
        reserved_for_response: 1000,
        min_messages_to_keep: 3,
        preserve_recent_count: 2,
        preserve_system_messages: true,
        preserve_tool_messages: false,
        ..Default::default()
    };

    let mut auto_compact = AutoCompact::new(config);

    let mut messages: Vec<LlmMessage> = vec![LlmMessage {
        role: MessageRole::System,
        content: "You are a helpful assistant.".to_string(),
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
        name: None,
        metadata: HashMap::new(),
    }];

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

    let result = auto_compact.force_compact(&mut messages).await?;

    assert!(result.was_compacted);
    assert!(result.compact_id.is_some());
    assert!(result.messages_after < result.messages_before);

    let has_boundary = messages.iter().any(is_compact_boundary);
    assert!(has_boundary);

    let sliced = slice_from_last_compact_boundary(&messages);
    assert!(sliced.len() <= messages.len());
    assert!(is_compact_boundary(&sliced[0]));

    // Add messages after compact
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

    let needs_compact = auto_compact.needs_compaction(&messages);
    assert!(!needs_compact);

    // Trigger second compact
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
    assert!(result2.was_compacted);
    assert_ne!(result.compact_id, result2.compact_id);

    let stats = auto_compact.stats();
    assert_eq!(stats.total_compactions, 2);
    assert!(stats.total_tokens_saved > 0);

    Ok(())
}

/// Test compact with custom instructions
#[tokio::test]
async fn test_compact_with_custom_instructions() -> SageResult<()> {
    use sage_core::context::{AutoCompact, AutoCompactConfig};

    let config = AutoCompactConfig {
        max_context_tokens: 500,
        reserved_for_response: 350,
        ..Default::default()
    };

    let mut auto_compact = AutoCompact::new(config);

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

    let result = auto_compact
        .compact_with_instructions(&mut messages, "Focus on TypeScript code changes")
        .await?;

    assert!(result.was_compacted);
    assert!(result.summary_preview.is_some());

    let preview = result.summary_preview.unwrap();
    assert!(!preview.is_empty());

    Ok(())
}

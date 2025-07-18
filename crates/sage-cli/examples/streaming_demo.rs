//! Streaming response demonstration
//! 
//! This example shows how to use the streaming LLM response feature
//! to provide real-time feedback to users.

use sage_core::{
    llm::{StreamChunk},
    llm::streaming::stream_utils,
    error::SageResult,
};
use futures::StreamExt;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🌊 Sage Agent Streaming Demo");
    println!("============================\n");

    // Note: This is a demonstration of the streaming API structure
    // In a real implementation, you would need valid API credentials
    println!("📋 1. Setting up streaming client");
    
    // Create a mock streaming demonstration
    demonstrate_streaming_concepts().await?;
    
    println!("\n🎉 Streaming demo completed!");
    println!("💡 Key benefits of streaming:");
    println!("   • Real-time user feedback");
    println!("   • Better perceived performance");
    println!("   • Ability to process partial responses");
    println!("   • Improved user experience for long responses");

    Ok(())
}

/// Demonstrate streaming concepts with mock data
async fn demonstrate_streaming_concepts() -> SageResult<()> {
    println!("🔄 2. Demonstrating streaming concepts");
    
    // Create a mock stream of chunks
    let mock_chunks = vec![
        StreamChunk::content("Hello"),
        StreamChunk::content(" there!"),
        StreamChunk::content(" I'm"),
        StreamChunk::content(" a"),
        StreamChunk::content(" streaming"),
        StreamChunk::content(" response."),
        StreamChunk::content(" This"),
        StreamChunk::content(" allows"),
        StreamChunk::content(" for"),
        StreamChunk::content(" real-time"),
        StreamChunk::content(" feedback!"),
        StreamChunk::final_chunk(
            Some(sage_core::types::LLMUsage {
                prompt_tokens: 20,
                completion_tokens: 15,
                total_tokens: 35,
                cost_usd: Some(0.001),
            }),
            Some("stop".to_string())
        ),
    ];

    println!("\n📡 Simulating streaming response:");
    print!("Response: ");
    io::stdout().flush().unwrap();

    // Simulate streaming with delays
    for chunk in mock_chunks {
        if let Some(content) = &chunk.content {
            print!("{}", content);
            io::stdout().flush().unwrap();
            
            // Simulate network delay
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        if chunk.is_final {
            println!("\n");
            if let Some(usage) = chunk.usage {
                println!("📊 Final usage: {} tokens (${:.4})", 
                    usage.total_tokens, 
                    usage.cost_usd.unwrap_or(0.0)
                );
            }
            if let Some(reason) = chunk.finish_reason {
                println!("🏁 Finished: {}", reason);
            }
        }
    }

    // Demonstrate stream utilities
    println!("\n🛠️  3. Stream utility functions");
    demonstrate_stream_utilities().await?;

    // Demonstrate SSE conversion
    println!("\n🌐 4. Server-Sent Events (SSE) support");
    demonstrate_sse_conversion().await?;

    Ok(())
}

/// Demonstrate stream utility functions
async fn demonstrate_stream_utilities() -> SageResult<()> {
    use futures::stream;

    // Create a mock stream
    let chunks = vec![
        Ok(StreamChunk::content("First ")),
        Ok(StreamChunk::content("chunk ")),
        Ok(StreamChunk::content("of ")),
        Ok(StreamChunk::content("content.")),
        Ok(StreamChunk::final_chunk(None, Some("stop".to_string()))),
    ];

    let stream = Box::pin(stream::iter(chunks));
    
    // Collect stream into complete response
    println!("🔄 Collecting stream into complete response...");
    let complete_response = stream_utils::collect_stream(stream).await?;
    println!("✅ Complete response: {}", complete_response.content);
    
    // Demonstrate content filtering
    let chunks2 = vec![
        Ok(StreamChunk::content("Content chunk")),
        Ok(StreamChunk::tool_calls(vec![])), // This will be filtered out
        Ok(StreamChunk::content(" more content")),
        Ok(StreamChunk::final_chunk(None, Some("stop".to_string()))),
    ];
    
    let stream2 = Box::pin(stream::iter(chunks2));
    let content_stream = stream_utils::content_only(stream2);
    
    println!("🔍 Content-only stream:");
    let mut content_stream = content_stream;
    while let Some(chunk_result) = content_stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(content) = chunk.content {
                    println!("  Content: '{}'", content);
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    Ok(())
}

/// Demonstrate SSE conversion
async fn demonstrate_sse_conversion() -> SageResult<()> {
    use sage_core::llm::streaming::sse;
    
    let chunks = vec![
        StreamChunk::content("Hello "),
        StreamChunk::content("world!"),
        StreamChunk::final_chunk(None, Some("stop".to_string())),
    ];
    
    println!("📡 Converting chunks to SSE format:");
    for (i, chunk) in chunks.iter().enumerate() {
        let sse_event = sse::chunk_to_sse(chunk.clone())?;
        println!("Event {}:", i + 1);
        println!("{}", sse_event);
    }

    Ok(())
}

/// Example of how to use streaming in a real application
#[allow(dead_code)]
async fn example_real_usage() -> SageResult<()> {
    // This is how you would use streaming in a real application
    // (requires valid API credentials)
    
    /*
    let config = Config::load_from_file("config.json")?;
    let client = LLMClient::new(
        LLMProvider::OpenAI,
        config.get_provider_config("openai")?,
        config.model_parameters.clone(),
    )?;
    
    let messages = vec![
        LLMMessage {
            role: MessageRole::User,
            content: "Write a short story about a robot learning to paint.".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }
    ];
    
    // Start streaming
    let mut stream = client.chat_stream(&messages, None).await?;
    
    print!("🤖 AI: ");
    io::stdout().flush().unwrap();
    
    // Process stream chunks
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(content) = chunk.content {
                    print!("{}", content);
                    io::stdout().flush().unwrap();
                }
                
                if chunk.is_final {
                    println!("\n");
                    if let Some(usage) = chunk.usage {
                        println!("📊 Usage: {} tokens", usage.total_tokens);
                    }
                    break;
                }
            }
            Err(e) => {
                eprintln!("\n❌ Stream error: {}", e);
                break;
            }
        }
    }
    */
    
    Ok(())
}

/// Example of integrating streaming with caching
#[allow(dead_code)]
async fn example_streaming_with_cache() -> SageResult<()> {
    /*
    use sage_core::cache::{CacheManager, CacheConfig, LLMCache};
    
    // Set up cache
    let cache_config = CacheConfig::default();
    let cache_manager = CacheManager::new(cache_config)?;
    let llm_cache = LLMCache::new(cache_manager, None);
    
    // Set up client
    let config = Config::load_from_file("config.json")?;
    let client = LLMClient::new(
        LLMProvider::OpenAI,
        config.get_provider_config("openai")?,
        config.model_parameters.clone(),
    )?;
    
    let messages = vec![
        LLMMessage {
            role: MessageRole::User,
            content: "Explain quantum computing".to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            metadata: HashMap::new(),
        }
    ];
    
    // Check cache first
    if let Some(cached_response) = llm_cache.get_response(
        "openai", 
        &client.model(), 
        &messages, 
        None
    ).await? {
        println!("📦 Using cached response: {}", cached_response.content);
        return Ok(());
    }
    
    // Stream if not cached
    let mut stream = client.chat_stream(&messages, None).await?;
    let mut full_content = String::new();
    
    print!("🤖 AI: ");
    io::stdout().flush().unwrap();
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(content) = &chunk.content {
                    print!("{}", content);
                    io::stdout().flush().unwrap();
                    full_content.push_str(content);
                }
                
                if chunk.is_final {
                    println!("\n");
                    
                    // Cache the complete response
                    let complete_response = sage_core::llm::LLMResponse {
                        content: full_content,
                        tool_calls: vec![],
                        usage: chunk.usage,
                        model: Some(client.model().to_string()),
                        finish_reason: chunk.finish_reason,
                        id: None,
                        metadata: HashMap::new(),
                    };
                    
                    llm_cache.cache_response(
                        "openai",
                        &client.model(),
                        &messages,
                        None,
                        &complete_response,
                        None,
                    ).await?;
                    
                    println!("💾 Response cached for future use");
                    break;
                }
            }
            Err(e) => {
                eprintln!("\n❌ Stream error: {}", e);
                break;
            }
        }
    }
    */
    
    Ok(())
}

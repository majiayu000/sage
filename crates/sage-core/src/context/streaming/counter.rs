//! Streaming token counter for real-time tracking

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::types::StreamingStats;

/// Counter for tracking tokens during streaming
#[derive(Debug)]
pub struct StreamingTokenCounter {
    /// Estimated characters per token
    chars_per_token: f32,
    /// Current character count
    char_count: AtomicUsize,
    /// Estimated token count
    estimated_tokens: AtomicUsize,
    /// Chunks received
    chunk_count: AtomicUsize,
    /// Start time
    start_time: RwLock<Option<Instant>>,
    /// First token time
    first_token_time: RwLock<Option<Instant>>,
}

impl StreamingTokenCounter {
    /// Create a new streaming counter
    pub fn new() -> Self {
        Self {
            chars_per_token: 4.0,
            char_count: AtomicUsize::new(0),
            estimated_tokens: AtomicUsize::new(0),
            chunk_count: AtomicUsize::new(0),
            start_time: RwLock::new(None),
            first_token_time: RwLock::new(None),
        }
    }

    /// Create with custom chars per token estimate
    pub fn with_chars_per_token(mut self, chars_per_token: f32) -> Self {
        self.chars_per_token = chars_per_token;
        self
    }

    /// Create for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        let chars_per_token = match provider.to_lowercase().as_str() {
            "anthropic" => 3.5,
            "openai" => 4.0,
            "google" => 4.0,
            _ => 4.0,
        };
        Self::new().with_chars_per_token(chars_per_token)
    }

    /// Start the counter (call before streaming begins)
    pub async fn start(&self) {
        let mut start = self.start_time.write().await;
        *start = Some(Instant::now());
    }

    /// Process a streaming chunk
    pub async fn process_chunk(&self, content: &str) {
        let chunk_chars = content.len();

        // Update char count
        self.char_count.fetch_add(chunk_chars, Ordering::SeqCst);

        // Update estimated tokens
        let total_chars = self.char_count.load(Ordering::SeqCst);
        let estimated = (total_chars as f32 / self.chars_per_token).ceil() as usize;
        self.estimated_tokens.store(estimated, Ordering::SeqCst);

        // Track chunk count
        let was_first = self.chunk_count.fetch_add(1, Ordering::SeqCst) == 0;

        // Record first token time
        if was_first {
            let mut first_time = self.first_token_time.write().await;
            if first_time.is_none() {
                *first_time = Some(Instant::now());
            }
        }
    }

    /// Get current estimated token count
    pub fn estimated_tokens(&self) -> usize {
        self.estimated_tokens.load(Ordering::SeqCst)
    }

    /// Get character count
    pub fn char_count(&self) -> usize {
        self.char_count.load(Ordering::SeqCst)
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunk_count.load(Ordering::SeqCst)
    }

    /// Get time to first token (TTFT)
    pub async fn time_to_first_token(&self) -> Option<Duration> {
        let start = self.start_time.read().await;
        let first = self.first_token_time.read().await;

        match (*start, *first) {
            (Some(s), Some(f)) => Some(f.duration_since(s)),
            _ => None,
        }
    }

    /// Get elapsed time since start
    pub async fn elapsed(&self) -> Option<Duration> {
        let start = self.start_time.read().await;
        start.map(|s| s.elapsed())
    }

    /// Get tokens per second rate
    pub async fn tokens_per_second(&self) -> Option<f64> {
        let elapsed = self.elapsed().await?;
        let tokens = self.estimated_tokens();
        let secs = elapsed.as_secs_f64();

        if secs > 0.0 {
            Some(tokens as f64 / secs)
        } else {
            None
        }
    }

    /// Get streaming statistics
    pub async fn stats(&self) -> StreamingStats {
        StreamingStats {
            char_count: self.char_count(),
            estimated_tokens: self.estimated_tokens(),
            chunk_count: self.chunk_count(),
            elapsed: self.elapsed().await,
            time_to_first_token: self.time_to_first_token().await,
            tokens_per_second: self.tokens_per_second().await,
        }
    }

    /// Reset the counter
    pub async fn reset(&self) {
        self.char_count.store(0, Ordering::SeqCst);
        self.estimated_tokens.store(0, Ordering::SeqCst);
        self.chunk_count.store(0, Ordering::SeqCst);
        *self.start_time.write().await = None;
        *self.first_token_time.write().await = None;
    }
}

impl Default for StreamingTokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

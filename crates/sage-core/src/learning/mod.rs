//! Learning mode for the Sage Agent
//!
//! This module provides a learning system that:
//! - Tracks user corrections and preferences
//! - Learns from tool usage patterns
//! - Detects coding style preferences
//! - Stores and applies learned patterns to improve agent behavior
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::learning::{LearningEngine, LearningConfig, Pattern, PatternType, PatternSource};
//!
//! // Create a learning engine
//! let engine = LearningEngine::new(LearningConfig::default());
//!
//! // Learn from a user correction
//! engine.learn_from_correction(
//!     "Using grep -r",
//!     "Use ripgrep (rg) for better performance",
//!     Some(vec!["bash".to_string()]),
//! ).await?;
//!
//! // Get patterns for system prompt
//! let patterns = engine.get_patterns_for_prompt(10).await;
//! ```

pub mod engine;
pub mod patterns;
pub mod types;

// Re-export main types from engine module
pub use engine::core::{
    LearningEngine, SharedLearningEngine, create_learning_engine,
    create_learning_engine_with_memory,
};
pub use engine::error::LearningError;

// Re-export from patterns module
pub use patterns::{
    CorrectionRecord, CorrectionStats, PatternDetector, PreferenceIndicator, StylePattern,
    analyze_user_message,
};

// Re-export from types module
pub use types::{
    Confidence, LearningConfig, LearningEvent, LearningEventType, LearningStats, Pattern,
    PatternId, PatternSource, PatternType,
};

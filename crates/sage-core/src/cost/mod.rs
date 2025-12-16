//! Cost tracking and pricing for LLM usage
//!
//! This module provides cost tracking and pricing information for LLM API calls.
//!
//! # Features
//!
//! - **Model Pricing**: Price definitions for various LLM providers and models
//! - **Usage Tracking**: Track token usage and calculate costs
//! - **Cost Limits**: Set spending limits with warnings
//! - **Statistics**: Aggregate stats by model, provider, and session
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::cost::{CostTracker, PricingRegistry};
//!
//! // Create tracker with default pricing
//! let tracker = CostTracker::new()
//!     .with_session("my-session")
//!     .with_cost_limit(10.0);
//!
//! // Track API call
//! let result = tracker.track("gpt-4o", 1000, 500).await;
//! println!("Cost: ${:.4}", result.record.cost);
//! println!("Cumulative: ${:.4}", result.cumulative_cost);
//!
//! // Check status
//! if result.status.is_warning() {
//!     println!("Approaching cost limit!");
//! }
//!
//! // Get statistics
//! let stats = tracker.get_stats().await;
//! println!("Total tokens: {}", stats.total_tokens());
//! println!("Total cost: {}", stats.format_cost());
//! ```

pub mod pricing;
pub mod tracker;

pub use pricing::{ModelPricing, PricingRegistry, TokenPrice};
pub use tracker::{
    CostStatus, CostTracker, ModelStats, ProviderStats, TrackResult, UsageRecord, UsageStats,
};

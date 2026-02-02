//! Metrics collection and aggregation for evaluation
//!
//! This module provides types and utilities for collecting and analyzing
//! evaluation metrics.

mod aggregator;
mod collector;
mod types;

pub use aggregator::MetricsAggregator;
pub use collector::MetricsCollector;
pub use types::{
    CategoryMetrics, CostEstimate, EvalMetrics, PassAtK, TaskResult, TaskStatus, TokenEfficiency,
    TurnMetrics,
};

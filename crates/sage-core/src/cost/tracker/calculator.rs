//! Cost calculation utilities

use super::types::{CostStatus, UsageStats};

/// Check cost status against limit
pub fn check_status(
    stats: &UsageStats,
    cost_limit: Option<f64>,
    warning_threshold: f64,
) -> CostStatus {
    match cost_limit {
        None => CostStatus::Ok,
        Some(limit) => {
            if stats.total_cost >= limit {
                CostStatus::LimitExceeded {
                    limit,
                    current: stats.total_cost,
                }
            } else if stats.total_cost >= limit * warning_threshold {
                CostStatus::Warning {
                    limit,
                    current: stats.total_cost,
                    threshold: warning_threshold,
                }
            } else {
                CostStatus::Ok
            }
        }
    }
}

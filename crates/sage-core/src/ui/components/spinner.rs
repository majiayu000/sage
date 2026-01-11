//! Spinner Component - Animated loading indicator
//!
//! Uses rnk's rendering to display animated spinner.

use crate::ui::theme::{Colors, Icons};
use rnk::prelude::*;
use std::time::{Duration, Instant};

/// Spinner component for loading animations
pub struct Spinner {
    color: Color,
    started_at: Instant,
}

impl Spinner {
    /// Create a new spinner with default settings
    pub fn new() -> Self {
        Self {
            color: Colors::TEXT_DIM,
            started_at: Instant::now(),
        }
    }

    /// Set the spinner color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the start time for animation synchronization
    pub fn started_at(mut self, time: Instant) -> Self {
        self.started_at = time;
        self
    }

    /// Get the current frame based on elapsed time
    fn current_frame(&self) -> &'static str {
        let frames = Icons::spinner_frames();
        let frame_duration = Duration::from_millis(80);
        let elapsed = self.started_at.elapsed();
        let frame_index = (elapsed.as_millis() / frame_duration.as_millis()) as usize % frames.len();
        frames[frame_index]
    }

    /// Render to Element
    pub fn into_element(self) -> Element {
        Text::new(self.current_frame())
            .color(self.color)
            .into_element()
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

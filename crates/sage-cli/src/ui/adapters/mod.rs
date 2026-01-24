//! UI Adapters - Framework-specific implementations of UI traits
//!
//! This module provides adapters that implement sage-core's UI traits
//! for specific UI frameworks.
//!
//! Currently supported:
//! - `RnkEventSink` - Adapter for the rnk terminal UI framework

mod rnk_event_sink;

pub use rnk_event_sink::RnkEventSink;

//! Team collaboration tools
//!
//! This module provides tools for multi-agent collaboration in a swarm:
//! - TeammateTool: Manage teams and coordinate teammates
//! - SendMessageTool: Send messages between teammates

pub mod send_message;
pub mod team_manager;
pub mod teammate;
pub mod types;

pub use send_message::SendMessageTool;
pub use team_manager::{TeamConfig, TeamManager, TeamMember};
pub use teammate::TeammateTool;

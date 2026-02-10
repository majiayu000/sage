//! SageComponents - All components built by SageBuilder

use crate::agent::lifecycle::{LifecycleHookRegistry, LifecycleManager};
use crate::concurrency::CancellationHierarchy;
use crate::config::model::Config;
use crate::error::SageResult;
use crate::events::EventBus;
use crate::mcp::McpRegistry;
use crate::tools::executor::ToolExecutor;
use crate::trajectory::SessionRecorder;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// All components built by SageBuilder
pub struct SageComponents {
    /// Tool executor for sequential tool execution
    pub tool_executor: ToolExecutor,
    /// Lifecycle manager with registered hooks
    pub lifecycle_manager: LifecycleManager,
    /// Event bus for pub/sub events
    pub event_bus: EventBus,
    /// Cancellation hierarchy for graceful shutdown
    pub cancellation: CancellationHierarchy,
    /// Optional session recorder
    pub session_recorder: Option<Arc<Mutex<SessionRecorder>>>,
    /// MCP registry for external tool servers
    pub mcp_registry: McpRegistry,
    /// Configuration
    pub config: Option<Config>,
    /// Max steps for agent execution
    pub max_steps: u32,
    /// Working directory
    pub working_dir: Option<PathBuf>,
}

impl SageComponents {
    /// Get a shared event bus
    pub fn shared_event_bus(&self) -> Arc<EventBus> {
        Arc::new(EventBus::new(1000))
    }

    /// Get lifecycle manager registry for adding more hooks
    pub fn lifecycle_registry(&self) -> Arc<LifecycleHookRegistry> {
        self.lifecycle_manager.registry()
    }

    /// Initialize the lifecycle manager
    pub async fn initialize(&self) -> SageResult<()> {
        let agent_id = uuid::Uuid::new_v4();
        self.lifecycle_manager.initialize(agent_id).await?;
        Ok(())
    }

    /// Shutdown all components
    pub async fn shutdown(&self) -> SageResult<()> {
        let agent_id = uuid::Uuid::new_v4();
        self.lifecycle_manager.shutdown(agent_id).await?;
        self.mcp_registry.close_all().await?;
        Ok(())
    }
}

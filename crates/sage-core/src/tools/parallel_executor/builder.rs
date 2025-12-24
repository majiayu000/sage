//! Builder pattern for ParallelToolExecutor

use std::sync::Arc;
use std::time::Duration;

use crate::tools::base::Tool;
use crate::tools::permission::{SharedPermissionHandler, ToolContext};

use super::config::ParallelExecutorConfig;
use super::executor::ParallelToolExecutor;

/// Builder for ParallelToolExecutor
pub struct ParallelExecutorBuilder {
    config: ParallelExecutorConfig,
    tools: Vec<Arc<dyn Tool>>,
    permission_handler: Option<SharedPermissionHandler>,
    context: Option<ToolContext>,
}

impl ParallelExecutorBuilder {
    pub fn new() -> Self {
        Self {
            config: ParallelExecutorConfig::default(),
            tools: Vec::new(),
            permission_handler: None,
            context: None,
        }
    }

    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.config.max_global_concurrency = max;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    pub fn with_permission_checking(mut self, enabled: bool) -> Self {
        self.config.check_permissions = enabled;
        self
    }

    pub fn with_permission_cache(mut self, enabled: bool) -> Self {
        self.config.use_permission_cache = enabled;
        self
    }

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn with_permission_handler(mut self, handler: SharedPermissionHandler) -> Self {
        self.permission_handler = Some(handler);
        self
    }

    pub fn with_context(mut self, context: ToolContext) -> Self {
        self.context = Some(context);
        self
    }

    pub async fn build(self) -> ParallelToolExecutor {
        let executor = ParallelToolExecutor::with_config(self.config);

        for tool in self.tools {
            executor.register_tool(tool);
        }

        if let Some(handler) = self.permission_handler {
            executor.set_permission_handler(handler).await;
        }

        if let Some(context) = self.context {
            executor.set_tool_context(context).await;
        }

        executor
    }
}

impl Default for ParallelExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

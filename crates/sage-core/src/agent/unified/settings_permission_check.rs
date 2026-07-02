use crate::tools::types::{ToolCall, ToolResult};

pub(in crate::agent::unified) enum SettingsPermissionCheck {
    Allowed(ToolCall),
    Blocked {
        result: ToolResult,
        tool_call: ToolCall,
    },
}

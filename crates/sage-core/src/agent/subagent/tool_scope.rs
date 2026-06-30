//! Tool scope resolution helpers for sub-agent roles.

use super::types::ToolAccessControl;

pub fn tool_allowed_by_parent(tool_name: &str, parent_tools: Option<&[String]>) -> bool {
    parent_tools
        .map(|tools| tools.iter().any(|tool| tool == tool_name))
        .unwrap_or(true)
}

pub fn resolve_tool_access(
    access: &ToolAccessControl,
    tool_name: &str,
    parent_tools: Option<&[String]>,
) -> bool {
    let parent_allows = || tool_allowed_by_parent(tool_name, parent_tools);
    match access {
        ToolAccessControl::All => parent_allows(),
        ToolAccessControl::Specific(tools) => {
            tools.iter().any(|tool| tool == tool_name) && parent_allows()
        }
        ToolAccessControl::None => false,
        ToolAccessControl::Inherited => parent_allows(),
        ToolAccessControl::InheritedRestricted(tools) => {
            tools.iter().any(|tool| tool == tool_name) && parent_allows()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent() -> Vec<String> {
        vec!["Read".to_string(), "Grep".to_string()]
    }

    #[test]
    fn subagent_tool_scope_intersects_all_with_parent_tools() {
        assert!(resolve_tool_access(
            &ToolAccessControl::All,
            "Read",
            Some(&parent())
        ));
        assert!(!resolve_tool_access(
            &ToolAccessControl::All,
            "Write",
            Some(&parent())
        ));
    }

    #[test]
    fn subagent_tool_scope_denies_specific_escalation() {
        let role_tools = ToolAccessControl::Specific(vec!["Read".to_string(), "Write".to_string()]);
        assert!(resolve_tool_access(&role_tools, "Read", Some(&parent())));
        assert!(!resolve_tool_access(&role_tools, "Write", Some(&parent())));
    }

    #[test]
    fn subagent_tool_scope_inherited_restricted_requires_both_sets() {
        let restricted =
            ToolAccessControl::InheritedRestricted(vec!["Read".to_string(), "Write".to_string()]);
        assert!(resolve_tool_access(&restricted, "Read", Some(&parent())));
        assert!(!resolve_tool_access(&restricted, "Write", Some(&parent())));
    }
}

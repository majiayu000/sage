//! Unified permission profile and decision engine.

mod approval_cache;
mod decision_engine;
mod decision_engine_keys;
mod profile;
mod profile_wire;
mod shell_safety;

pub use approval_cache::{ApprovalCache, ApprovalCacheDecision, ApprovalCacheLookup};
pub use decision_engine::{
    PermissionAction, PermissionDecision, PermissionDecisionEngine, PermissionDecisionInput,
    PermissionDecisionKind, PermissionPreflight, SandboxSupport,
};
pub(crate) use decision_engine_keys::permission_pattern_matches;
pub use profile::{
    ApprovalPermissionProfile, ExecPermissionProfile, FilesystemPermissionProfile,
    NetworkPermissionProfile, PermissionBehavior, PermissionDomainSources, PermissionProfile,
    PermissionProfileSource, PermissionRule, SandboxPermissionProfile,
};

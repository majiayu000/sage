use super::profile::{
    ApprovalPermissionProfile, ExecPermissionProfile, FilesystemPermissionProfile,
    NetworkPermissionProfile, PermissionBehavior, PermissionDomainSources, PermissionProfile,
    PermissionProfileSource, PermissionRule, SandboxPermissionProfile,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Deserialize)]
struct PermissionRuleWire {
    pattern: String,
    source: Option<PermissionProfileSource>,
}

impl PermissionRuleWire {
    fn into_rule(self, source: PermissionProfileSource) -> PermissionRule {
        PermissionRule {
            pattern: self.pattern,
            source: capped_source(self.source, source),
        }
    }
}

#[derive(Deserialize)]
struct PermissionProfileWire {
    source: Option<PermissionProfileSource>,
    #[serde(default)]
    filesystem: Option<FilesystemPermissionProfile>,
    #[serde(default)]
    network: Option<NetworkPermissionProfile>,
    #[serde(default)]
    exec: Option<ExecPermissionProfile>,
    #[serde(default)]
    sandbox: Option<SandboxPermissionProfile>,
    #[serde(default)]
    approval: Option<ApprovalPermissionProfile>,
    #[serde(default)]
    allow: Vec<PermissionRuleWire>,
    #[serde(default)]
    deny: Vec<PermissionRuleWire>,
    #[serde(default)]
    default_behavior: Option<PermissionBehavior>,
    #[serde(default)]
    default_behavior_set: bool,
    #[serde(default)]
    default_behavior_source: Option<PermissionProfileSource>,
    #[serde(default)]
    domain_sources: PermissionDomainSources,
}

#[derive(Serialize)]
struct PermissionProfileSerializeWire<'a> {
    source: PermissionProfileSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    filesystem: Option<&'a FilesystemPermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<&'a NetworkPermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exec: Option<&'a ExecPermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox: Option<&'a SandboxPermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval: Option<&'a ApprovalPermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow: Option<&'a [PermissionRule]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deny: Option<&'a [PermissionRule]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_behavior: Option<PermissionBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_behavior_source: Option<PermissionProfileSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain_sources: Option<&'a PermissionDomainSources>,
}

impl<'de> Deserialize<'de> for PermissionProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PermissionProfileWire::deserialize(deserializer)?;
        let source = wire
            .source
            .ok_or_else(|| de::Error::missing_field("source"))?;
        let filesystem_set = wire.filesystem.is_some();
        let network_set = wire.network.is_some();
        let exec_set = wire.exec.is_some();
        let sandbox_set = wire.sandbox.is_some();
        let approval_set = wire.approval.is_some();
        let mut domain_sources = PermissionDomainSources::default();
        if filesystem_set {
            domain_sources.filesystem = Some(capped_source(wire.domain_sources.filesystem, source));
        }
        if network_set {
            domain_sources.network = Some(capped_source(wire.domain_sources.network, source));
        }
        if exec_set {
            domain_sources.exec = Some(capped_source(wire.domain_sources.exec, source));
        }
        if sandbox_set {
            domain_sources.sandbox = Some(capped_source(wire.domain_sources.sandbox, source));
        }
        if approval_set {
            domain_sources.approval = Some(capped_source(wire.domain_sources.approval, source));
        }

        let default_behavior_present = wire.default_behavior.is_some();
        let default_behavior = wire.default_behavior.unwrap_or_default();
        let default_behavior_set = wire.default_behavior_set || default_behavior_present;
        let default_behavior_source =
            default_behavior_set.then(|| capped_source(wire.default_behavior_source, source));
        Ok(PermissionProfile {
            source,
            filesystem: wire.filesystem.unwrap_or_default(),
            network: wire.network.unwrap_or_default(),
            exec: wire.exec.unwrap_or_default(),
            sandbox: wire.sandbox.unwrap_or_default(),
            approval: wire.approval.unwrap_or_default(),
            allow: wire
                .allow
                .into_iter()
                .map(|rule| rule.into_rule(source))
                .collect(),
            deny: wire
                .deny
                .into_iter()
                .map(|rule| rule.into_rule(source))
                .collect(),
            default_behavior,
            default_behavior_set,
            default_behavior_source,
            domain_sources,
        })
    }
}

fn capped_source(
    claimed: Option<PermissionProfileSource>,
    fragment_source: PermissionProfileSource,
) -> PermissionProfileSource {
    match claimed {
        Some(claimed) if claimed.precedence() <= fragment_source.precedence() => claimed,
        _ => fragment_source,
    }
}

impl Serialize for PermissionProfile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let has_default_behavior =
            self.default_behavior_set || self.default_behavior != PermissionBehavior::Ask;
        let wire = PermissionProfileSerializeWire {
            source: self.source,
            filesystem: self
                .domain_sources
                .filesystem
                .is_some()
                .then_some(&self.filesystem),
            network: self
                .domain_sources
                .network
                .is_some()
                .then_some(&self.network),
            exec: self.domain_sources.exec.is_some().then_some(&self.exec),
            sandbox: self
                .domain_sources
                .sandbox
                .is_some()
                .then_some(&self.sandbox),
            approval: self
                .domain_sources
                .approval
                .is_some()
                .then_some(&self.approval),
            allow: (!self.allow.is_empty()).then_some(self.allow.as_slice()),
            deny: (!self.deny.is_empty()).then_some(self.deny.as_slice()),
            default_behavior: has_default_behavior.then_some(self.default_behavior),
            default_behavior_source: has_default_behavior
                .then_some(self.default_behavior_source)
                .flatten(),
            domain_sources: (!self.domain_sources.is_empty()).then_some(&self.domain_sources),
        };
        wire.serialize(serializer)
    }
}

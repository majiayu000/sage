use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::envelope::RuntimeEnvelope;
use super::permission::{RuntimePermissionDecision, RuntimeRule};

pub type RuntimeRequest = RuntimeEnvelope<RuntimeRequestPayload>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeRequestPayload {
    ThreadStart(RuntimeThreadStartPayload),
    ThreadResume(RuntimeThreadResumePayload),
    ThreadFork(RuntimeThreadForkPayload),
    TurnStart(RuntimeTurnStartPayload),
    TurnSteer(RuntimeTurnSteerPayload),
    TurnInterrupt(RuntimeTurnInterruptPayload),
    PermissionRespond(RuntimePermissionRespondPayload),
    InputRespond(RuntimeInputRespondPayload),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeExecutionMode {
    Interactive,
    NonInteractive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeForkMode {
    FullContext,
    Summary,
    SelectedItems,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeThreadStartPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RuntimeExecutionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeThreadResumePayload {
    pub thread_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_latest: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeThreadForkPayload {
    pub parent_thread_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_mode: Option<RuntimeForkMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTurnStartPayload {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTurnSteerPayload {
    pub instructions: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTurnInterruptPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePermissionRespondPayload {
    pub decision: RuntimePermissionDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_input: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuntimeRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeInputRespondPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<bool>,
}

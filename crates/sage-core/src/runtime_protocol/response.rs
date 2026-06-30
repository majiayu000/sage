use serde::{Deserialize, Serialize};

use super::envelope::RuntimeEnvelope;

pub type RuntimeResponse = RuntimeEnvelope<RuntimeResponsePayload>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeResponsePayload {
    Ack(RuntimeAckResponsePayload),
    Thread(RuntimeThreadResponsePayload),
    Turn(RuntimeTurnResponsePayload),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeAckResponsePayload {
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_rules: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeThreadResponsePayload {
    pub thread_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resumed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTurnResponsePayload {
    pub turn_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

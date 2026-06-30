use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ops::Deref;
use thiserror::Error;

use super::envelope::{RuntimeEnvelope, RuntimeKind};
use super::permission::{RuntimePermissionDecision, RuntimeRule};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RuntimeRequest(pub RuntimeEnvelope<RuntimeRequestPayload>);

impl From<RuntimeEnvelope<RuntimeRequestPayload>> for RuntimeRequest {
    fn from(envelope: RuntimeEnvelope<RuntimeRequestPayload>) -> Self {
        Self(envelope)
    }
}

impl Deref for RuntimeRequest {
    type Target = RuntimeEnvelope<RuntimeRequestPayload>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RuntimeRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RuntimeEnvelope::<Value>::deserialize(deserializer)?;
        if raw.kind != RuntimeKind::Request {
            return Err(serde::de::Error::custom("runtime request kind mismatch"));
        }

        let payload = request_payload_from_type(&raw.message_type, raw.payload)
            .map_err(serde::de::Error::custom)?;

        Ok(RuntimeRequest(RuntimeEnvelope {
            protocol_version: raw.protocol_version,
            kind: raw.kind,
            message_type: raw.message_type,
            id: raw.id,
            thread_id: raw.thread_id,
            turn_id: raw.turn_id,
            item_id: raw.item_id,
            request_id: raw.request_id,
            timestamp: raw.timestamp,
            sequence: raw.sequence,
            source: raw.source,
            payload,
            metadata: raw.metadata,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Error)]
enum RuntimeRequestPayloadDecodeError {
    #[error("unsupported runtime request type {0}")]
    UnsupportedType(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn request_payload_from_type(
    message_type: &str,
    payload: Value,
) -> Result<RuntimeRequestPayload, RuntimeRequestPayloadDecodeError> {
    match message_type {
        "thread.start" => decode_payload(payload, RuntimeRequestPayload::ThreadStart),
        "thread.resume" => decode_payload(payload, RuntimeRequestPayload::ThreadResume),
        "thread.fork" => decode_payload(payload, RuntimeRequestPayload::ThreadFork),
        "turn.start" => decode_payload(payload, RuntimeRequestPayload::TurnStart),
        "turn.steer" => decode_payload(payload, RuntimeRequestPayload::TurnSteer),
        "turn.interrupt" => decode_payload(payload, RuntimeRequestPayload::TurnInterrupt),
        "permission.respond" => decode_payload(payload, RuntimeRequestPayload::PermissionRespond),
        "input.respond" => decode_payload(payload, RuntimeRequestPayload::InputRespond),
        _ => Err(RuntimeRequestPayloadDecodeError::UnsupportedType(
            message_type.to_string(),
        )),
    }
}

fn decode_payload<T>(
    payload: Value,
    wrap: impl FnOnce(T) -> RuntimeRequestPayload,
) -> Result<RuntimeRequestPayload, RuntimeRequestPayloadDecodeError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload)
        .map(wrap)
        .map_err(Into::into)
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
    pub modified_input: Option<BTreeMap<String, Value>>,
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

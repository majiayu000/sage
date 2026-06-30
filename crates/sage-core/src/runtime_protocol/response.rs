use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;
use thiserror::Error;

use super::envelope::{RuntimeEnvelope, RuntimeKind};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RuntimeResponse(pub RuntimeEnvelope<RuntimeResponsePayload>);

impl From<RuntimeEnvelope<RuntimeResponsePayload>> for RuntimeResponse {
    fn from(envelope: RuntimeEnvelope<RuntimeResponsePayload>) -> Self {
        Self(envelope)
    }
}

impl Deref for RuntimeResponse {
    type Target = RuntimeEnvelope<RuntimeResponsePayload>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RuntimeResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RuntimeEnvelope::<Value>::deserialize(deserializer)?;
        if raw.kind != RuntimeKind::Response {
            return Err(serde::de::Error::custom("runtime response kind mismatch"));
        }

        let payload = response_payload_from_type(&raw.message_type, raw.payload)
            .map_err(serde::de::Error::custom)?;

        Ok(RuntimeResponse(RuntimeEnvelope {
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
pub enum RuntimeResponsePayload {
    Ack(RuntimeAckResponsePayload),
    Thread(RuntimeThreadResponsePayload),
    Turn(RuntimeTurnResponsePayload),
}

#[derive(Debug, Error)]
enum RuntimeResponsePayloadDecodeError {
    #[error("unsupported runtime response type {0}")]
    UnsupportedType(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn response_payload_from_type(
    message_type: &str,
    payload: Value,
) -> Result<RuntimeResponsePayload, RuntimeResponsePayloadDecodeError> {
    match message_type {
        "thread.start.result" | "thread.resume.result" | "thread.fork.result" => {
            decode_payload(payload, RuntimeResponsePayload::Thread)
        }
        "turn.start.result" | "turn.steer.result" | "turn.interrupt.result" => {
            decode_payload(payload, RuntimeResponsePayload::Turn)
        }
        "permission.respond.result" | "input.respond.result" => {
            decode_payload(payload, RuntimeResponsePayload::Ack)
        }
        _ => Err(RuntimeResponsePayloadDecodeError::UnsupportedType(
            message_type.to_string(),
        )),
    }
}

fn decode_payload<T>(
    payload: Value,
    wrap: impl FnOnce(T) -> RuntimeResponsePayload,
) -> Result<RuntimeResponsePayload, RuntimeResponsePayloadDecodeError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload)
        .map(wrap)
        .map_err(Into::into)
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

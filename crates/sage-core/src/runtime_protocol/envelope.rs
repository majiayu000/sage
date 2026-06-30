use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::error::RuntimeError;
use super::notification::RuntimeNotification;
use super::request::RuntimeRequest;
use super::response::RuntimeResponse;

pub type RuntimeMetadata = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeProtocolVersion {
    #[serde(rename = "sage.runtime.v0")]
    V0,
}

impl Default for RuntimeProtocolVersion {
    fn default() -> Self {
        Self::V0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    Request,
    Notification,
    Response,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSource {
    Cli,
    Sdk,
    Runtime,
    Tool,
    Permission,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvelope<P> {
    pub protocol_version: RuntimeProtocolVersion,
    pub kind: RuntimeKind,
    #[serde(rename = "type")]
    pub message_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    pub source: RuntimeSource,
    pub payload: P,
    #[serde(default, skip_serializing_if = "RuntimeMetadata::is_empty")]
    pub metadata: RuntimeMetadata,
}

impl<P> RuntimeEnvelope<P> {
    pub fn new(
        kind: RuntimeKind,
        message_type: impl Into<String>,
        id: impl Into<String>,
        timestamp: DateTime<Utc>,
        source: RuntimeSource,
        payload: P,
    ) -> Self {
        Self {
            protocol_version: RuntimeProtocolVersion::V0,
            kind,
            message_type: message_type.into(),
            id: id.into(),
            thread_id: None,
            turn_id: None,
            item_id: None,
            request_id: None,
            timestamp,
            sequence: None,
            source,
            payload,
            metadata: RuntimeMetadata::new(),
        }
    }

    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }

    pub fn with_turn_id(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }

    pub fn with_item_id(mut self, item_id: impl Into<String>) -> Self {
        self.item_id = Some(item_id.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = Some(sequence);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RuntimeMessage {
    Request(RuntimeRequest),
    Notification(RuntimeNotification),
    Response(RuntimeResponse),
    Error(RuntimeError),
}

impl<'de> Deserialize<'de> for RuntimeMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("runtime message missing kind"))?;

        match kind {
            "request" => serde_json::from_value(value)
                .map(Self::Request)
                .map_err(serde::de::Error::custom),
            "notification" => serde_json::from_value(value)
                .map(Self::Notification)
                .map_err(serde::de::Error::custom),
            "response" => serde_json::from_value(value)
                .map(Self::Response)
                .map_err(serde::de::Error::custom),
            "error" => serde_json::from_value(value)
                .map(Self::Error)
                .map_err(serde::de::Error::custom),
            other => Err(serde::de::Error::custom(format!(
                "unsupported runtime message kind {other}"
            ))),
        }
    }
}

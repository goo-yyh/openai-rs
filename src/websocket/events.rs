//! WebSocket event models and lifecycle message types.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::error::WebSocketError;

/// 表示服务端推送的通用 WebSocket 事件。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WebSocketServerEvent {
    /// 事件类型。
    #[serde(rename = "type", default)]
    pub event_type: String,
    /// 除 `type` 外的原始负载字段。
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

impl WebSocketServerEvent {
    /// 判断当前事件是否为错误事件。
    pub fn is_error(&self) -> bool {
        self.event_type == "error"
    }

    /// 尝试从错误事件中提取可读错误消息。
    pub fn error_message(&self) -> Option<String> {
        self.data
            .get("error")
            .and_then(|value| {
                value
                    .get("message")
                    .or_else(|| value.get("error"))
                    .or_else(|| value.get("detail"))
            })
            .or_else(|| self.data.get("message"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

/// 表示响应创建事件。
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseCreatedEvent {
    /// 响应 ID。
    pub id: Option<String>,
    /// 原始响应对象。
    pub response: Option<Value>,
    /// 原始事件。
    pub raw: WebSocketServerEvent,
}

/// 表示输出文本增量事件。
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseOutputTextDeltaEvent {
    /// 文本增量。
    pub delta: Option<String>,
    /// 响应 ID。
    pub response_id: Option<String>,
    /// 输出项 ID。
    pub item_id: Option<String>,
    /// 原始事件。
    pub raw: WebSocketServerEvent,
}

/// 表示会话创建事件。
#[derive(Debug, Clone, PartialEq)]
pub struct SessionCreatedEvent {
    /// 会话 ID。
    pub id: Option<String>,
    /// 原始会话对象。
    pub session: Option<Value>,
    /// 原始事件。
    pub raw: WebSocketServerEvent,
}

/// Realtime 服务端事件。
#[derive(Debug, Clone, PartialEq)]
pub enum RealtimeServerEvent {
    /// 会话创建事件。
    SessionCreated(SessionCreatedEvent),
    /// 响应创建事件。
    ResponseCreated(ResponseCreatedEvent),
    /// 输出文本增量事件。
    ResponseOutputTextDelta(ResponseOutputTextDeltaEvent),
    /// 未知事件，保留原始负载以保证向前兼容。
    Unknown(WebSocketServerEvent),
}

/// Responses 服务端事件。
#[derive(Debug, Clone, PartialEq)]
pub enum ResponsesServerEvent {
    /// 响应创建事件。
    ResponseCreated(ResponseCreatedEvent),
    /// 输出文本增量事件。
    ResponseOutputTextDelta(ResponseOutputTextDeltaEvent),
    /// 未知事件，保留原始负载以保证向前兼容。
    Unknown(WebSocketServerEvent),
}

impl RealtimeServerEvent {
    /// 返回事件类型。
    pub fn event_type(&self) -> &str {
        self.raw().event_type.as_str()
    }

    /// 返回原始事件。
    pub fn raw(&self) -> &WebSocketServerEvent {
        match self {
            Self::SessionCreated(event) => &event.raw,
            Self::ResponseCreated(event) => &event.raw,
            Self::ResponseOutputTextDelta(event) => &event.raw,
            Self::Unknown(event) => event,
        }
    }
}

impl ResponsesServerEvent {
    /// 返回事件类型。
    pub fn event_type(&self) -> &str {
        self.raw().event_type.as_str()
    }

    /// 返回原始事件。
    pub fn raw(&self) -> &WebSocketServerEvent {
        match self {
            Self::ResponseCreated(event) => &event.raw,
            Self::ResponseOutputTextDelta(event) => &event.raw,
            Self::Unknown(event) => event,
        }
    }
}

impl From<WebSocketServerEvent> for RealtimeServerEvent {
    fn from(raw: WebSocketServerEvent) -> Self {
        match raw.event_type.as_str() {
            "session.created" => Self::SessionCreated(SessionCreatedEvent {
                id: extract_event_string(&raw, "id").or_else(|| {
                    raw.data
                        .get("session")
                        .and_then(|value| value.get("id"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
                session: raw.data.get("session").cloned(),
                raw,
            }),
            "response.created" => Self::ResponseCreated(ResponseCreatedEvent {
                id: extract_event_string(&raw, "id").or_else(|| {
                    raw.data
                        .get("response")
                        .and_then(|value| value.get("id"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
                response: raw.data.get("response").cloned(),
                raw,
            }),
            "response.output_text.delta" => {
                Self::ResponseOutputTextDelta(ResponseOutputTextDeltaEvent {
                    delta: extract_event_string(&raw, "delta"),
                    response_id: extract_event_string(&raw, "response_id"),
                    item_id: extract_event_string(&raw, "item_id"),
                    raw,
                })
            }
            _ => Self::Unknown(raw),
        }
    }
}

impl From<WebSocketServerEvent> for ResponsesServerEvent {
    fn from(raw: WebSocketServerEvent) -> Self {
        match raw.event_type.as_str() {
            "response.created" => Self::ResponseCreated(ResponseCreatedEvent {
                id: extract_event_string(&raw, "id").or_else(|| {
                    raw.data
                        .get("response")
                        .and_then(|value| value.get("id"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
                response: raw.data.get("response").cloned(),
                raw,
            }),
            "response.output_text.delta" => {
                Self::ResponseOutputTextDelta(ResponseOutputTextDeltaEvent {
                    delta: extract_event_string(&raw, "delta"),
                    response_id: extract_event_string(&raw, "response_id"),
                    item_id: extract_event_string(&raw, "item_id"),
                    raw,
                })
            }
            _ => Self::Unknown(raw),
        }
    }
}

impl<'de> Deserialize<'de> for RealtimeServerEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        WebSocketServerEvent::deserialize(deserializer).map(Self::from)
    }
}

impl<'de> Deserialize<'de> for ResponsesServerEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        WebSocketServerEvent::deserialize(deserializer).map(Self::from)
    }
}

impl Serialize for RealtimeServerEvent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.raw().serialize(serializer)
    }
}

impl Serialize for ResponsesServerEvent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.raw().serialize(serializer)
    }
}

fn extract_event_string(raw: &WebSocketServerEvent, key: &str) -> Option<String> {
    raw.data.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// 表示 WebSocket 流中的生命周期或消息事件。
#[derive(Debug, Clone)]
pub enum SocketStreamMessage<T> {
    /// 连接正在建立。
    Connecting,
    /// 连接已建立。
    Open,
    /// 连接正在关闭。
    Closing,
    /// 连接已经关闭。
    Close,
    /// 收到服务端消息。
    Message(T),
    /// 收到协议层或业务层错误。
    Error(WebSocketError),
}

/// Realtime WebSocket 流消息。
pub type RealtimeStreamMessage = SocketStreamMessage<RealtimeServerEvent>;

/// Responses WebSocket 流消息。
pub type ResponsesStreamMessage = SocketStreamMessage<ResponsesServerEvent>;

/// 表示关闭 WebSocket 时附带的参数。
#[derive(Debug, Clone)]
pub struct SocketCloseOptions {
    /// WebSocket close code。
    pub code: u16,
    /// 关闭原因。
    pub reason: String,
}

impl Default for SocketCloseOptions {
    fn default() -> Self {
        Self {
            code: 1000,
            reason: "OK".into(),
        }
    }
}

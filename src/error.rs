//! SDK 错误类型定义。

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::providers::ProviderKind;

/// SDK 统一 `Result` 类型别名。
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// SDK 对外暴露的统一错误类型。
#[derive(Debug, Error)]
pub enum Error {
    /// 表示客户端配置无效。
    #[error("客户端配置无效: {0}")]
    InvalidConfig(String),
    /// 表示请求缺少必填字段。
    #[error("请求缺少必填字段: {field}")]
    MissingRequiredField {
        /// 缺失字段名。
        field: &'static str,
    },
    /// 表示缺少请求所需的凭证。
    #[error("缺少 API 凭证")]
    MissingCredentials,
    /// 表示接口返回了业务错误。
    #[error(transparent)]
    Api(#[from] ApiError),
    /// 表示底层网络连接相关错误。
    #[error(transparent)]
    Connection(#[from] ConnectionError),
    /// 表示请求执行超时。
    #[error("请求超时")]
    Timeout,
    /// 表示流式解析相关错误。
    #[error(transparent)]
    Stream(#[from] StreamError),
    /// 表示 WebSocket 错误。
    #[error(transparent)]
    WebSocket(#[from] WebSocketError),
    /// 表示序列化或反序列化失败。
    #[error(transparent)]
    Serialization(#[from] SerializationError),
    /// 表示模型因长度截断而无法完成结构化语义。
    #[error(transparent)]
    LengthFinishReason(#[from] LengthFinishReasonError),
    /// 表示模型因内容过滤而无法完成结构化语义。
    #[error(transparent)]
    ContentFilterFinishReason(#[from] ContentFilterFinishReasonError),
    /// 表示 Webhook 校验失败。
    #[error(transparent)]
    WebhookVerification(#[from] WebhookVerificationError),
    /// 表示当前 Provider 的兼容性校验失败。
    #[error(transparent)]
    ProviderCompatibility(#[from] ProviderCompatibilityError),
    /// 表示请求被主动取消。
    #[error("请求已取消")]
    Cancelled,
}

/// 表示 API 错误的大类。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorKind {
    /// 表示 400 类错误。
    BadRequest,
    /// 表示鉴权失败。
    Authentication,
    /// 表示权限不足。
    PermissionDenied,
    /// 表示资源不存在。
    NotFound,
    /// 表示资源冲突。
    Conflict,
    /// 表示请求参数语义错误。
    UnprocessableEntity,
    /// 表示触发限流。
    RateLimit,
    /// 表示服务端内部错误。
    InternalServer,
    /// 表示未归类的 API 错误。
    Unknown,
}

impl ApiErrorKind {
    /// 根据 HTTP 状态码推导错误大类。
    pub fn from_status(status: u16) -> Self {
        match status {
            400 => Self::BadRequest,
            401 => Self::Authentication,
            403 => Self::PermissionDenied,
            404 => Self::NotFound,
            409 => Self::Conflict,
            422 => Self::UnprocessableEntity,
            429 => Self::RateLimit,
            500..=599 => Self::InternalServer,
            _ => Self::Unknown,
        }
    }
}

/// 表示标准化后的 API 错误对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// HTTP 状态码。
    pub status: u16,
    /// 错误大类。
    pub kind: ApiErrorKind,
    /// 主要错误消息。
    pub message: String,
    /// 请求 ID。
    pub request_id: Option<String>,
    /// 当前 Provider。
    pub provider: ProviderKind,
    /// 原始错误载荷。
    pub raw: Option<Value>,
}

impl ApiError {
    /// 创建一个新的 API 错误。
    pub fn new(
        status: u16,
        message: impl Into<String>,
        request_id: Option<String>,
        provider: ProviderKind,
        raw: Option<Value>,
    ) -> Self {
        Self {
            status,
            kind: ApiErrorKind::from_status(status),
            message: message.into(),
            request_id,
            provider,
            raw,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (status {})", self.message, self.status)
    }
}

impl std::error::Error for ApiError {}

/// 表示底层连接或 DNS/TLS 等错误。
#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct ConnectionError {
    /// 错误消息。
    pub message: String,
}

impl ConnectionError {
    /// 创建新的连接错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 表示序列化或反序列化错误。
#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct SerializationError {
    /// 错误消息。
    pub message: String,
}

impl SerializationError {
    /// 创建新的序列化错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 表示 SSE 或增量聚合相关错误。
#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct StreamError {
    /// 错误消息。
    pub message: String,
}

/// 表示模型因为达到长度上限而提前终止。
#[derive(Debug, Error, Clone)]
#[error("无法继续解析响应内容: 模型因长度上限提前结束")]
pub struct LengthFinishReasonError;

/// 表示模型输出被内容过滤器拦截。
#[derive(Debug, Error, Clone)]
#[error("无法继续解析响应内容: 请求被内容过滤器拦截")]
pub struct ContentFilterFinishReasonError;

impl StreamError {
    /// 创建新的流式错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 表示 WebSocket 连接或协议错误。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebSocketErrorKind {
    /// 传输层错误，例如底层连接或 IO 问题。
    Transport,
    /// 协议层错误，例如事件反序列化失败。
    Protocol,
    /// 服务端主动推送的错误事件。
    Server,
}

/// 表示 WebSocket 连接或协议错误。
#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct WebSocketError {
    /// 错误分类。
    pub kind: WebSocketErrorKind,
    /// 错误消息。
    pub message: String,
    /// 关联的服务端事件类型。
    pub event_type: Option<String>,
}

impl WebSocketError {
    /// 创建新的 WebSocket 错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: WebSocketErrorKind::Protocol,
            message: message.into(),
            event_type: None,
        }
    }

    /// 创建新的传输层 WebSocket 错误。
    pub fn transport(message: impl Into<String>) -> Self {
        Self {
            kind: WebSocketErrorKind::Transport,
            message: message.into(),
            event_type: None,
        }
    }

    /// 创建新的协议层 WebSocket 错误。
    pub fn protocol(message: impl Into<String>) -> Self {
        Self {
            kind: WebSocketErrorKind::Protocol,
            message: message.into(),
            event_type: None,
        }
    }

    /// 创建新的服务端 WebSocket 错误。
    pub fn server(message: impl Into<String>, event_type: Option<String>) -> Self {
        Self {
            kind: WebSocketErrorKind::Server,
            message: message.into(),
            event_type,
        }
    }
}

/// 表示 Webhook 校验错误。
#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct WebhookVerificationError {
    /// 错误消息。
    pub message: String,
}

impl WebhookVerificationError {
    /// 创建新的 Webhook 校验错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 表示 Provider 兼容性错误。
#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct ProviderCompatibilityError {
    /// 错误消息。
    pub message: String,
    /// 触发错误的 Provider。
    pub provider: ProviderKind,
}

impl ProviderCompatibilityError {
    /// 创建新的 Provider 兼容性错误。
    pub fn new(provider: ProviderKind, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            provider,
        }
    }
}

/// 表示通用的 API 错误载荷结构。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorBody {
    /// 错误消息。
    pub message: Option<String>,
    /// 错误类型。
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    /// 错误参数。
    pub param: Option<String>,
    /// 错误码。
    pub code: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

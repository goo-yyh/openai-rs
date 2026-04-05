#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(rust_2024_compatibility, missing_debug_implementations)]

//! `openai-rs` 提供了一个围绕 OpenAI 兼容接口构建的异步 Rust SDK。
//! 它支持多 Provider、分页、SSE 流、Multipart 上传、Webhook 校验以及工具调用辅助能力。

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod files;
pub mod helpers;
pub mod pagination;
pub mod providers;
pub mod resource;
pub mod resources;
pub mod response_meta;
pub mod stream;
pub mod transport;
pub mod webhooks;
pub mod websocket;

pub use auth::ApiKeySource;
pub use client::{Client, ClientBuilder};
pub use config::{ClientOptions, RequestOptions};
pub use error::{
    ApiError, ApiErrorKind, ConnectionError, Error, ProviderCompatibilityError, Result,
    SerializationError, StreamError, WebSocketError, WebhookVerificationError,
};
pub use files::{FileLike, MultipartField, UploadSource};
pub use helpers::{
    ParsedChatCompletion, ParsedResponse, ToolDefinition, ToolHandler, ToolRegistry,
    json_schema_for, parse_json_payload,
};
pub use pagination::{CursorPage, ListEnvelope, Page, PageStream};
pub use providers::{
    AuthScheme, AzureAuthMode, AzureOptions, CapabilitySet, CompatibilityMode, Provider,
    ProviderKind, ProviderProfile,
};
pub use resources::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionMessage, ChatCompletionToolCall,
    DeleteResponse, EmbeddingResponse, FileObject, Model, Response, UploadObject,
};
pub use response_meta::{ApiResponse, ResponseMeta};
pub use stream::{
    ChatCompletionStream, LineDecoder, RawSseStream, ResponseStream, SseEvent, SseStream,
};
pub use webhooks::{HeaderLookup, WebhookEvent, WebhookVerifier};
pub use websocket::{
    RealtimeServerEvent, RealtimeSocket, RealtimeStreamMessage, ResponsesServerEvent,
    ResponsesSocket, ResponsesStreamMessage, SocketCloseOptions, SocketStreamMessage,
    WebSocketServerEvent,
};

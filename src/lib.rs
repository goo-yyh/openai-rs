#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(rust_2024_compatibility, missing_debug_implementations)]

//! `openai-rs` 提供了一个围绕 OpenAI 兼容接口构建的异步 Rust SDK。
//! 它支持多 Provider、分页、SSE 流、Multipart 上传、Webhook 校验以及工具调用辅助能力。

pub mod audio_helpers;
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod files;
#[cfg(feature = "structured-output")]
#[cfg_attr(docsrs, doc(cfg(feature = "structured-output")))]
pub mod helpers;
pub mod pagination;
pub mod providers;
pub mod resources;
mod response_meta;
pub mod stream;
mod transport;
pub mod webhooks;
#[cfg(any(feature = "realtime", feature = "responses-ws"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "realtime", feature = "responses-ws"))))]
pub mod websocket;

pub use audio_helpers::{AudioPlaybackInput, RecordAudioOptions, play_audio, record_audio};
pub use auth::ApiKeySource;
pub use client::{Client, ClientBuilder};
pub use config::{ClientOptions, LogLevel, LogRecord, Logger, LoggerHandle, RequestOptions};
pub use error::{
    ApiError, ApiErrorKind, ConnectionError, ContentFilterFinishReasonError, Error,
    LengthFinishReasonError, ProviderCompatibilityError, Result, SerializationError, StreamError,
    WebSocketError, WebSocketErrorKind, WebhookVerificationError,
};
pub use files::{FileLike, MultipartField, ToFileInput, UploadSource, to_file};
#[cfg(feature = "structured-output")]
#[cfg_attr(docsrs, doc(cfg(feature = "structured-output")))]
pub use helpers::{ParsedChatCompletion, ParsedResponse, json_schema_for, parse_json_payload};
#[cfg(feature = "tool-runner")]
#[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
pub use helpers::{ToolDefinition, ToolHandler, ToolRegistry};
pub use pagination::{CursorPage, ListEnvelope, Page, PageStream};
pub use providers::{
    AuthScheme, AzureAuthMode, AzureOptions, CapabilitySet, CompatibilityMode, Provider,
    ProviderKind, ProviderProfile,
};
pub use resources::{
    BetaAssistant, BetaThread, BetaThreadMessage, BetaThreadRun, BetaThreadRunStep, ChatCompletion,
    ChatCompletionChunk, ChatCompletionMessage, ChatCompletionToolCall, ChatContentDeltaEvent,
    ChatLogProbsDeltaEvent, ChatRefusalDeltaEvent, ChatToolArgumentsDeltaEvent, DeleteResponse,
    EmbeddingResponse, FileObject, Model, Response, UploadObject, VectorStore, VectorStoreFile,
    VectorStoreFileBatch, VectorStoreSearchResponse,
};
#[cfg(feature = "tool-runner")]
#[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
pub use resources::{
    ChatCompletionRunner, ChatCompletionStreamingRunner, ChatCompletionToolResult,
};
pub use response_meta::{ApiResponse, ResponseMeta};
pub use stream::{
    AssistantEventStream, AssistantImageFileDoneEvent, AssistantMessageCreatedEvent,
    AssistantMessageDeltaEvent, AssistantMessageDoneEvent, AssistantRunStepCreatedEvent,
    AssistantRunStepDeltaEvent, AssistantRunStepDoneEvent, AssistantRuntimeEvent, AssistantStream,
    AssistantStreamEvent, AssistantStreamSnapshot, AssistantTextCreatedEvent,
    AssistantTextDeltaEvent, AssistantTextDoneEvent, AssistantToolCallCreatedEvent,
    AssistantToolCallDeltaEvent, AssistantToolCallDoneEvent, ChatCompletionEventStream,
    ChatCompletionRuntimeEvent, ChatCompletionStream, ChatContentDoneEvent,
    ChatContentSnapshotEvent, ChatLogProbsDoneEvent, ChatLogProbsSnapshotEvent,
    ChatRefusalDoneEvent, ChatRefusalSnapshotEvent, ChatToolArgumentsDoneEvent,
    ChatToolArgumentsSnapshotEvent, LineDecoder, RawSseStream, ResponseEventStream,
    ResponseFunctionCallArgumentsEvent, ResponseOutputTextEvent, ResponseRuntimeEvent,
    ResponseStream, SseEvent, SseStream,
};
pub use webhooks::{HeaderLookup, WebhookEvent, WebhookVerifier};
#[cfg(feature = "responses-ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "responses-ws")))]
pub use websocket::OpenAIResponsesWebSocket;
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub use websocket::{OpenAIRealtimeWS, OpenAIRealtimeWebSocket};
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub use websocket::{RealtimeServerEvent, RealtimeSocket, RealtimeStreamMessage};
#[cfg(feature = "responses-ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "responses-ws")))]
pub use websocket::{ResponsesServerEvent, ResponsesSocket, ResponsesStreamMessage};
#[cfg(any(feature = "realtime", feature = "responses-ws"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "realtime", feature = "responses-ws"))))]
pub use websocket::{SocketCloseOptions, SocketStreamMessage, WebSocketServerEvent};

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
mod generated;
#[cfg(feature = "structured-output")]
#[cfg_attr(docsrs, doc(cfg(feature = "structured-output")))]
pub mod helpers;
mod json_payload;
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
pub use json_payload::JsonPayload;
pub use pagination::{CursorPage, ListEnvelope, Page, PageStream};
pub use providers::{
    AuthScheme, AzureAuthMode, AzureOptions, CapabilitySet, CompatibilityMode, Provider,
    ProviderKind, ProviderProfile,
};
pub use resources::{
    AudioSpeechCreateParams, AudioTranscription, AudioTranscriptionSegment,
    AudioTranscriptionSegmentId, AudioTranscriptionWord, AudioTranslation, Batch,
    BatchCreateParams, BatchError, BatchErrors, BatchRequestCounts, BatchUsage,
    BatchUsageInputTokensDetails, BatchUsageOutputTokensDetails, BetaAssistant, BetaAssistantTool,
    BetaRealtimeSession, BetaRealtimeTranscriptionSession, BetaThread, BetaThreadMessage,
    BetaThreadMessageContent, BetaThreadRun, BetaThreadRunIncompleteDetails,
    BetaThreadRunLastError, BetaThreadRunRequiredAction, BetaThreadRunRequiredActionFunction,
    BetaThreadRunRequiredActionFunctionToolCall, BetaThreadRunRequiredActionSubmitToolOutputs,
    BetaThreadRunStep, BetaThreadRunStepDetails, BetaThreadRunTool, BetaThreadRunUsage,
    BetaThreadToolResources, ChatCompletion, ChatCompletionChoiceLogprobs, ChatCompletionChunk,
    ChatCompletionMessage, ChatCompletionStoreContentPart, ChatCompletionStoreMessage,
    ChatCompletionTokenLogprob, ChatCompletionTokenTopLogprob, ChatCompletionToolCall,
    ChatContentDeltaEvent, ChatKitConfiguration, ChatKitRateLimits, ChatKitSession, ChatKitThread,
    ChatKitThreadContent, ChatKitThreadItem, ChatKitThreadStatus, ChatKitWorkflow,
    ChatLogProbsDeltaEvent, ChatReasoningDetail, ChatRefusalDeltaEvent,
    ChatToolArgumentsDeltaEvent, ChatToolChoice, ChatToolChoiceMode, Completion, CompletionChoice,
    CompletionLogProbs, CompletionUsage, CompletionUsageCompletionTokensDetails,
    CompletionUsagePromptTokensDetails, Container, ContainerCreateParams, ContainerExpiresAfter,
    ContainerFile, ContainerFileCreateParams, Conversation, ConversationContentPart,
    ConversationCreateParams, ConversationInputItem, ConversationItem,
    ConversationItemCreateParams, ConversationUpdateParams, DeleteResponse, EmbeddingData,
    EmbeddingResponse, EmbeddingUsage, Eval, EvalCreateParams, EvalDataSourceConfig, EvalOutput,
    EvalOutputItem, EvalRun, EvalRunCreateParams, EvalRunInput, EvalTestingCriterion,
    EvalUpdateParams, FileObject, FineTuningCheckpoint, FineTuningCheckpointPermission,
    FineTuningHyperparameterValue, FineTuningJob, FineTuningJobCreateParams, FineTuningJobError,
    FineTuningJobEvent, FineTuningJobHyperparameters, FineTuningJobIntegration, FineTuningMetrics,
    FineTuningWandbIntegration, GraderModel, GraderModelCatalog, GraderRunErrors,
    GraderRunMetadata, GraderRunResponse, GraderValidateResponse, ImageData, ImageGenerateParams,
    ImageGenerationResponse, InputTokenCount, KnownResponseOutputTextAnnotation, Model,
    ModerationCreateResponse, ModerationResult, RealtimeClientSecretCreateResponse,
    RealtimeSessionClientSecret, RealtimeSessionPayload, Response, ResponseError,
    ResponseFunctionToolCall, ResponseIncompleteDetails, ResponseInputItemPayload,
    ResponseInputPayload, ResponseInputTokensDetails, ResponseOutputContentPart,
    ResponseOutputContentPartRaw, ResponseOutputItem, ResponseOutputItemRaw, ResponseOutputMessage,
    ResponseOutputRefusal, ResponseOutputText, ResponseOutputTextAnnotation,
    ResponseOutputTextAnnotationUnknown, ResponseOutputTextContainerFileCitation,
    ResponseOutputTextFileCitation, ResponseOutputTextFilePath, ResponseOutputTextLogprob,
    ResponseOutputTextTopLogprob, ResponseOutputTextUrlCitation, ResponseOutputTokensDetails,
    ResponseUsage, Skill, SkillCreateParams, SkillUpdateParams, SkillVersion, SkillVersionContent,
    SkillVersionCreateParams, UploadObject, UploadPart, VectorStore, VectorStoreAttributeValue,
    VectorStoreAttributes, VectorStoreExpiresAfter, VectorStoreFile, VectorStoreFileBatch,
    VectorStoreFileChunkingStrategy, VectorStoreFileContent, VectorStoreFileCounts,
    VectorStoreFileLastError, VectorStoreMetadata, VectorStoreSearchContent,
    VectorStoreSearchResponse, VectorStoreSearchResult, VectorStoreStaticFileChunkingStrategy,
    Video, VideoCharacter, VideoCharacterCreateParams, VideoCreateParams,
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

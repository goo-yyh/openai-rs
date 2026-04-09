//! 资源命名空间、公开类型与请求构建器。

mod audio;
mod batches;
mod beta;
mod chat;
mod common;
mod containers;
mod conversations;
mod core;
mod evals;
mod files;
mod fine_tuning;
mod images;
mod longtail;
mod responses;
mod skills;
mod uploads;
mod vector_stores;
mod videos;
mod webhooks;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Client;
use crate::error::{Error, Result};
#[cfg(feature = "tool-runner")]
use crate::helpers::ToolDefinition;
use crate::json_payload::JsonPayload;

pub use beta::{
    BetaAssistant, BetaAssistantTool, BetaRealtimeSession, BetaRealtimeTranscriptionSession,
    BetaThread, BetaThreadMessage, BetaThreadMessageContent, BetaThreadRun,
    BetaThreadRunIncompleteDetails, BetaThreadRunLastError, BetaThreadRunRequiredAction,
    BetaThreadRunRequiredActionFunction, BetaThreadRunRequiredActionFunctionToolCall,
    BetaThreadRunRequiredActionSubmitToolOutputs, BetaThreadRunStep, BetaThreadRunStepDetails,
    BetaThreadRunTool, BetaThreadRunUsage, BetaThreadToolResources, ChatKitConfiguration,
    ChatKitRateLimits, ChatKitSession, ChatKitThread, ChatKitThreadContent, ChatKitThreadItem,
    ChatKitThreadStatus, ChatKitWorkflow,
};
#[cfg(feature = "structured-output")]
pub use chat::ChatCompletionParseRequestBuilder;
pub use chat::{
    AssistantStreamRequestBuilder, ChatCompletionCreateRequestBuilder, ChatCompletionStoreMessage,
    ChatCompletionStreamRequestBuilder,
};
#[cfg(feature = "tool-runner")]
pub use chat::{
    ChatCompletionRunToolsRequestBuilder, ChatCompletionRunner, ChatCompletionStreamingRunner,
    ChatCompletionToolResult,
};
pub use common::{
    BytesRequestBuilder, JsonRequestBuilder, ListRequestBuilder, NoContentRequestBuilder,
};
pub(crate) use common::{
    TypedJsonRequestState, encode_path_segment, metadata_is_empty, value_from,
};
pub use core::{
    Completion, CompletionChoice, CompletionLogProbs, CompletionUsage,
    CompletionUsageCompletionTokensDetails, CompletionUsagePromptTokensDetails,
    ModerationCreateResponse, ModerationResult,
};
pub use fine_tuning::{
    GraderModel, GraderModelCatalog, GraderRunErrors, GraderRunMetadata, GraderRunResponse,
    GraderValidateResponse,
};
pub use longtail::{
    AudioSpeechCreateParams, AudioSpeechRequestBuilder, AudioTranscription,
    AudioTranscriptionRequestBuilder, AudioTranscriptionSegment, AudioTranscriptionSegmentId,
    AudioTranscriptionWord, AudioTranslation, AudioTranslationRequestBuilder, Batch,
    BatchCreateParams, BatchCreateRequestBuilder, BatchError, BatchErrors, BatchRequestCounts,
    BatchUsage, BatchUsageInputTokensDetails, BatchUsageOutputTokensDetails, Container,
    ContainerCreateParams, ContainerExpiresAfter, ContainerFile, ContainerFileCreateParams,
    Conversation, ConversationContentPart, ConversationCreateParams, ConversationInputItem,
    ConversationItem, ConversationItemCreateParams, ConversationUpdateParams, Eval,
    EvalCreateParams, EvalDataSourceConfig, EvalOutput, EvalOutputItem, EvalRun,
    EvalRunCreateParams, EvalRunInput, EvalTestingCriterion, EvalUpdateParams,
    FineTuningCheckpoint, FineTuningCheckpointPermission, FineTuningHyperparameterValue,
    FineTuningJob, FineTuningJobCreateParams, FineTuningJobCreateRequestBuilder,
    FineTuningJobError, FineTuningJobEvent, FineTuningJobHyperparameters, FineTuningJobIntegration,
    FineTuningMetrics, FineTuningWandbIntegration, ImageData, ImageGenerateParams,
    ImageGenerateRequestBuilder, ImageGenerationResponse, Skill, SkillCreateParams,
    SkillUpdateParams, SkillVersion, SkillVersionContent, SkillVersionCreateParams, Video,
    VideoCharacter, VideoCharacterCreateParams, VideoCreateParams,
};
#[cfg(feature = "realtime")]
pub use responses::RealtimeSocketRequestBuilder;
#[cfg(feature = "structured-output")]
pub use responses::ResponseParseRequestBuilder;
#[cfg(feature = "responses-ws")]
pub use responses::ResponsesSocketRequestBuilder;
pub use responses::{
    RealtimeClientSecretCreateResponse, RealtimeSessionClientSecret, ResponseCreateRequestBuilder,
    ResponseStreamRequestBuilder,
};
pub use uploads::UploadPart;
pub use vector_stores::{
    VectorStore, VectorStoreAttributeValue, VectorStoreAttributes, VectorStoreExpiresAfter,
    VectorStoreFile, VectorStoreFileBatch, VectorStoreFileChunkingStrategy, VectorStoreFileContent,
    VectorStoreFileCounts, VectorStoreFileLastError, VectorStoreMetadata, VectorStoreSearchContent,
    VectorStoreSearchResponse, VectorStoreSearchResult, VectorStoreStaticFileChunkingStrategy,
};

macro_rules! json_payload_wrapper {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Value);

        impl Default for $name {
            fn default() -> Self {
                Self(Value::Null)
            }
        }

        impl From<Value> for $name {
            fn from(value: Value) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Value {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl $name {
            /// 返回未经解释的原始 JSON 值。
            pub fn as_raw(&self) -> &Value {
                &self.0
            }

            /// 消费该包装器并返回原始 JSON 值。
            pub fn into_raw(self) -> Value {
                self.0
            }

            /// 返回载荷中的 `type` 字段，若存在且为字符串。
            pub fn kind(&self) -> Option<&str> {
                self.0.get("type").and_then(Value::as_str)
            }
        }
    };
}

macro_rules! handle {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name {
            client: Client,
        }

        impl $name {
            pub(crate) fn new(client: Client) -> Self {
                Self { client }
            }
        }
    };
}

/// 表示常见的删除结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeleteResponse {
    /// 被删除对象的 ID。
    pub id: Option<String>,
    /// 是否删除成功。
    #[serde(default)]
    pub deleted: bool,
    /// 对象类型。
    pub object: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示模型对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model {
    /// 模型 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 模型所有者。
    pub owned_by: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示文件对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileObject {
    /// 文件 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 文件名。
    pub filename: Option<String>,
    /// 文件用途。
    pub purpose: Option<String>,
    /// 文件大小。
    pub bytes: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示上传对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadObject {
    /// 上传 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 上传状态。
    pub status: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 Embeddings 接口的返回值。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingResponse {
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 向量数据。
    #[serde(default)]
    pub data: Vec<EmbeddingData>,
    /// 使用统计。
    pub usage: Option<EmbeddingUsage>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示单个 embedding 向量项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingData {
    /// embedding 向量。
    #[serde(default)]
    pub embedding: Vec<f64>,
    /// 向量索引。
    pub index: Option<u32>,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 embeddings 的用量统计。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingUsage {
    /// prompt token 数。
    #[serde(default)]
    pub prompt_tokens: u64,
    /// 总 token 数。
    #[serde(default)]
    pub total_tokens: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示模型计数结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputTokenCount {
    /// 总 token 数量。
    pub total_tokens: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

json_payload_wrapper!(
    /// 表示已存储 chat completion 的 content part。
    ChatCompletionStoreContentPart
);
json_payload_wrapper!(
    /// 表示聊天消息中的 reasoning detail。
    ChatReasoningDetail
);
json_payload_wrapper!(
    /// 表示 chat 请求中的 tool_choice 载荷。
    ChatToolChoice
);
json_payload_wrapper!(
    /// 表示 responses 请求的输入载荷。
    ResponseInputPayload
);
json_payload_wrapper!(
    /// 表示 responses 请求中的单个输入项。
    ResponseInputItemPayload
);
json_payload_wrapper!(
    /// 表示 realtime client secret 返回中的 session 配置。
    RealtimeSessionPayload
);
json_payload_wrapper!(
    /// 表示 responses 输出中的未知原始输出项。
    ResponseOutputItemRaw
);
json_payload_wrapper!(
    /// 表示 responses message 中的未知原始内容片段。
    ResponseOutputContentPartRaw
);

/// 表示 chat tool_choice 的字符串模式。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatToolChoiceMode {
    /// 不允许工具调用。
    None,
    /// 允许模型自动选择。
    Auto,
    /// 要求模型必须调用工具。
    Required,
}

impl From<ChatToolChoiceMode> for ChatToolChoice {
    fn from(mode: ChatToolChoiceMode) -> Self {
        let value = match mode {
            ChatToolChoiceMode::None => Value::String("none".into()),
            ChatToolChoiceMode::Auto => Value::String("auto".into()),
            ChatToolChoiceMode::Required => Value::String("required".into()),
        };
        Self::from(value)
    }
}

impl From<String> for ResponseInputPayload {
    fn from(value: String) -> Self {
        Self::from(Value::String(value))
    }
}

impl From<&str> for ResponseInputPayload {
    fn from(value: &str) -> Self {
        Self::from(Value::String(value.into()))
    }
}

impl From<Vec<ResponseInputItemPayload>> for ResponseInputPayload {
    fn from(items: Vec<ResponseInputItemPayload>) -> Self {
        Self::from(Value::Array(items.into_iter().map(Value::from).collect()))
    }
}

impl ChatToolChoice {
    /// 生成 `none` 模式。
    pub fn none() -> Self {
        ChatToolChoiceMode::None.into()
    }

    /// 生成 `auto` 模式。
    pub fn auto() -> Self {
        ChatToolChoiceMode::Auto.into()
    }

    /// 生成 `required` 模式。
    pub fn required() -> Self {
        ChatToolChoiceMode::Required.into()
    }

    /// 强制调用指定函数工具。
    pub fn function(name: impl Into<String>) -> Self {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": name.into(),
            },
        })
        .into()
    }

    /// 强制调用指定自定义工具。
    pub fn custom(name: impl Into<String>) -> Self {
        serde_json::json!({
            "type": "custom",
            "custom": {
                "name": name.into(),
            },
        })
        .into()
    }

    /// 当 tool_choice 为字符串模式时返回该模式。
    pub fn mode_name(&self) -> Option<&str> {
        self.0.as_str()
    }
}

/// 表示一个工具函数调用。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionFunctionCall {
    /// 工具名称。
    pub name: String,
    /// 参数 JSON 字符串。
    #[serde(default)]
    pub arguments: String,
}

/// 表示工具调用项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionToolCall {
    /// 工具调用 ID。
    pub id: String,
    /// 调用类型。
    #[serde(rename = "type", default = "default_function_type")]
    pub call_type: String,
    /// 函数调用内容。
    pub function: ChatCompletionFunctionCall,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 token 的 top logprob。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ChatCompletionTokenTopLogprob {
    /// token 内容。
    #[serde(default)]
    pub token: String,
    /// UTF-8 bytes。
    pub bytes: Option<Vec<u8>>,
    /// token 对应的 logprob。
    #[serde(default)]
    pub logprob: f64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示单个 token 的 logprob 信息。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ChatCompletionTokenLogprob {
    /// token 内容。
    #[serde(default)]
    pub token: String,
    /// UTF-8 bytes。
    pub bytes: Option<Vec<u8>>,
    /// token 对应的 logprob。
    #[serde(default)]
    pub logprob: f64,
    /// top logprobs。
    #[serde(default)]
    pub top_logprobs: Vec<ChatCompletionTokenTopLogprob>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 chat completion choice 的 token logprobs。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ChatCompletionChoiceLogprobs {
    /// 内容 token 的 logprobs。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ChatCompletionTokenLogprob>,
    /// refusal token 的 logprobs。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusal: Vec<ChatCompletionTokenLogprob>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatCompletionChoiceLogprobs {
    /// 按字段名返回对应的 token logprobs 列表。
    pub fn values(&self, field_name: &str) -> Option<&[ChatCompletionTokenLogprob]> {
        match field_name {
            "content" if !self.content.is_empty() => Some(&self.content),
            "refusal" if !self.refusal.is_empty() => Some(&self.refusal),
            _ => None,
        }
    }
}

/// 表示流式函数调用增量。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionFunctionCallDelta {
    /// 函数名称增量。
    pub name: Option<String>,
    /// 参数增量。
    pub arguments: Option<String>,
}

/// 表示流式工具调用增量。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionToolCallDelta {
    /// 增量对应索引。
    pub index: Option<u32>,
    /// 工具调用 ID。
    pub id: Option<String>,
    /// 调用类型。
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    /// 函数调用增量。
    pub function: Option<ChatCompletionFunctionCallDelta>,
}

/// 表示聊天消息。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionMessage {
    /// 角色。
    pub role: String,
    /// 文本内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// 可选名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 工具调用关联 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 工具调用集合。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ChatCompletionToolCall>,
    /// 拒绝回答文本。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// 推理内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// 推理细节。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasoning_details: Vec<ChatReasoningDetail>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatCompletionMessage {
    /// 创建 system 消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: Some(content.into()),
            ..Self::default()
        }
    }

    /// 创建 user 消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            ..Self::default()
        }
    }

    /// 创建 assistant 消息。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(content.into()),
            ..Self::default()
        }
    }

    /// 创建 tool 消息。
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Self::default()
        }
    }

    /// 尝试把消息文本解析为结构化对象。
    ///
    /// # Errors
    ///
    /// 当文本存在但 JSON 解析失败时返回错误。
    pub fn parse_content<T>(&self) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.content
            .as_deref()
            .map(parse_jsonish_payload)
            .transpose()
    }

    /// 尝试解析首个工具调用的参数。
    ///
    /// # Errors
    ///
    /// 当工具参数不是合法 JSON 时返回错误。
    pub fn parse_tool_arguments<T>(&self) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.tool_calls
            .first()
            .map(|tool_call| parse_json_arguments(&tool_call.function.arguments))
            .transpose()
    }

    /// 尝试解析指定工具调用的参数。
    ///
    /// # Errors
    ///
    /// 当工具参数不是合法 JSON 时返回错误。
    pub fn parse_tool_arguments_by_id<T>(&self, tool_call_id: &str) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.tool_calls
            .iter()
            .find(|tool_call| tool_call.id == tool_call_id)
            .map(|tool_call| parse_json_arguments(&tool_call.function.arguments))
            .transpose()
    }
}

/// 表示聊天补全中的单个候选项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChoice {
    /// 候选项索引。
    pub index: u32,
    /// 结束原因。
    pub finish_reason: Option<String>,
    /// 返回消息。
    pub message: ChatCompletionMessage,
    /// token 级 logprobs。
    pub logprobs: Option<ChatCompletionChoiceLogprobs>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示聊天补全结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletion {
    /// 补全 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 创建时间。
    pub created: Option<i64>,
    /// 模型 ID。
    #[serde(default)]
    pub model: String,
    /// 候选项集合。
    #[serde(default)]
    pub choices: Vec<ChatCompletionChoice>,
    /// 使用统计。
    pub usage: Option<CompletionUsage>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatCompletion {
    /// 校验返回结果没有因为长度或内容过滤而失去可解析语义。
    ///
    /// # Errors
    ///
    /// 当任一 choice 的 `finish_reason` 为 `length` 或 `content_filter` 时返回错误。
    pub fn ensure_not_truncated(&self) -> Result<&Self> {
        for choice in &self.choices {
            match choice.finish_reason.as_deref() {
                Some("length") => return Err(crate::LengthFinishReasonError.into()),
                Some("content_filter") => return Err(crate::ContentFilterFinishReasonError.into()),
                _ => {}
            }
        }
        Ok(self)
    }
}

/// 表示流式增量。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunkDelta {
    /// 角色增量。
    pub role: Option<String>,
    /// 文本内容增量。
    pub content: Option<String>,
    /// 拒绝回答文本增量。
    pub refusal: Option<String>,
    /// 推理内容增量。
    pub reasoning_content: Option<String>,
    /// 推理细节增量。
    #[serde(default)]
    pub reasoning_details: Vec<ChatReasoningDetail>,
    /// 工具调用增量。
    #[serde(default)]
    pub tool_calls: Vec<ChatCompletionToolCallDelta>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示流式候选项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunkChoice {
    /// 候选索引。
    pub index: u32,
    /// 增量载荷。
    pub delta: ChatCompletionChunkDelta,
    /// 结束原因。
    pub finish_reason: Option<String>,
    /// token 级 logprobs 增量。
    pub logprobs: Option<ChatCompletionChoiceLogprobs>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示聊天文本增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatContentDeltaEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 文本增量。
    pub delta: String,
}

/// 表示聊天拒绝回答增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatRefusalDeltaEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 拒绝文本增量。
    pub delta: String,
}

/// 表示工具参数增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatToolArgumentsDeltaEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 工具调用索引。
    pub tool_call_index: u32,
    /// 工具名称增量。
    pub name: Option<String>,
    /// 参数增量。
    pub delta: String,
}

/// 表示 token logprobs 增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatLogProbsDeltaEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// logprobs 明细。
    pub values: Vec<ChatCompletionTokenLogprob>,
}

/// 表示聊天补全 SSE 分片。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// 分片所属补全 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 创建时间。
    pub created: Option<i64>,
    /// 模型 ID。
    #[serde(default)]
    pub model: String,
    /// 候选项集合。
    #[serde(default)]
    pub choices: Vec<ChatCompletionChunkChoice>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatCompletionChunk {
    /// 提取所有文本内容增量。
    pub fn content_deltas(&self) -> Vec<ChatContentDeltaEvent> {
        self.choices
            .iter()
            .filter_map(|choice| {
                choice
                    .delta
                    .content
                    .as_ref()
                    .map(|delta| ChatContentDeltaEvent {
                        choice_index: choice.index,
                        delta: delta.clone(),
                    })
            })
            .collect()
    }

    /// 提取所有拒绝回答增量。
    pub fn refusal_deltas(&self) -> Vec<ChatRefusalDeltaEvent> {
        self.choices
            .iter()
            .filter_map(|choice| {
                choice
                    .delta
                    .refusal
                    .as_ref()
                    .map(|delta| ChatRefusalDeltaEvent {
                        choice_index: choice.index,
                        delta: delta.clone(),
                    })
            })
            .collect()
    }

    /// 提取所有工具参数增量。
    pub fn tool_argument_deltas(&self) -> Vec<ChatToolArgumentsDeltaEvent> {
        self.choices
            .iter()
            .flat_map(|choice| {
                choice.delta.tool_calls.iter().filter_map(|tool_call| {
                    let delta = tool_call.function.as_ref()?.arguments.clone()?;
                    Some(ChatToolArgumentsDeltaEvent {
                        choice_index: choice.index,
                        tool_call_index: tool_call.index.unwrap_or_default(),
                        name: tool_call
                            .function
                            .as_ref()
                            .and_then(|function| function.name.clone()),
                        delta,
                    })
                })
            })
            .collect()
    }

    /// 提取内容 token 的 logprobs 增量。
    pub fn logprobs_content_deltas(&self) -> Vec<ChatLogProbsDeltaEvent> {
        extract_logprobs(self, "content")
    }

    /// 提取拒绝回答 token 的 logprobs 增量。
    pub fn logprobs_refusal_deltas(&self) -> Vec<ChatLogProbsDeltaEvent> {
        extract_logprobs(self, "refusal")
    }
}

fn extract_logprobs(chunk: &ChatCompletionChunk, field_name: &str) -> Vec<ChatLogProbsDeltaEvent> {
    chunk
        .choices
        .iter()
        .filter_map(|choice| {
            let values = choice
                .logprobs
                .as_ref()
                .and_then(|logprobs| logprobs.values(field_name))?
                .to_vec();
            Some(ChatLogProbsDeltaEvent {
                choice_index: choice.index,
                values,
            })
        })
        .collect()
}

/// 表示聊天工具定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolDefinition {
    /// 工具类型，当前固定为 `function`。
    #[serde(rename = "type")]
    pub tool_type: String,
    /// 函数定义。
    pub function: ChatToolFunction,
}

impl ChatToolDefinition {
    #[cfg(feature = "tool-runner")]
    fn from_tool(tool: &ToolDefinition) -> Self {
        Self {
            tool_type: "function".into(),
            function: ChatToolFunction {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: tool.parameters.clone(),
            },
        }
    }

    /// 转换为 Responses API 所需的扁平工具定义格式。
    fn as_response_tool_value(&self) -> Value {
        serde_json::json!({
            "type": self.tool_type,
            "name": self.function.name,
            "description": self.function.description,
            "parameters": self.function.parameters,
        })
    }
}

/// 表示聊天工具函数定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolFunction {
    /// 函数名。
    pub name: String,
    /// 函数描述。
    pub description: Option<String>,
    /// 参数 Schema。
    pub parameters: JsonPayload,
}

/// 表示聊天补全请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionCreateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 历史消息。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<ChatCompletionMessage>,
    /// 温度。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 候选数量。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// 最大 token 数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 工具定义。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ChatToolDefinition>,
    /// 工具选择策略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ChatToolChoice>,
    /// 流式开关。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// 表示 Responses 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Response {
    /// 响应 ID。
    pub id: String,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 模型 ID。
    pub model: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 错误信息。
    pub error: Option<ResponseError>,
    /// 不完整原因。
    pub incomplete_details: Option<ResponseIncompleteDetails>,
    /// 元数据。
    pub metadata: Option<BTreeMap<String, String>>,
    /// 输出项。
    #[serde(default)]
    pub output: Vec<ResponseOutputItem>,
    /// 用量统计。
    pub usage: Option<ResponseUsage>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Response {
    /// 尝试提取最终文本输出。
    pub fn output_text(&self) -> Option<String> {
        for item in &self.output {
            if let Some(text) = item.output_text() {
                return Some(text.to_owned());
            }
        }

        self.extra
            .get("output_text")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

/// 表示 responses 顶层错误。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseError {
    /// 错误码。
    pub code: Option<String>,
    /// 错误消息。
    pub message: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 responses 不完整原因。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseIncompleteDetails {
    /// 不完整原因。
    pub reason: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 responses 用量明细。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseUsage {
    /// 输入 token 数。
    #[serde(default)]
    pub input_tokens: u64,
    /// 输入 token 明细。
    pub input_tokens_details: Option<ResponseInputTokensDetails>,
    /// 输出 token 数。
    #[serde(default)]
    pub output_tokens: u64,
    /// 输出 token 明细。
    pub output_tokens_details: Option<ResponseOutputTokensDetails>,
    /// 总 token 数。
    #[serde(default)]
    pub total_tokens: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 responses 输入 token 明细。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseInputTokensDetails {
    /// cache 命中 token 数。
    pub cached_tokens: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 responses 输出 token 明细。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseOutputTokensDetails {
    /// reasoning token 数。
    pub reasoning_tokens: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示响应输出项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseOutputItem {
    /// 已知输出项。
    Known(KnownResponseOutputItem),
    /// 向前兼容保留的原始项。
    Raw(ResponseOutputItemRaw),
}

impl Default for ResponseOutputItem {
    fn default() -> Self {
        Self::Raw(ResponseOutputItemRaw::default())
    }
}

impl ResponseOutputItem {
    /// 返回消息输出项。
    pub fn as_message(&self) -> Option<&ResponseOutputMessage> {
        match self {
            Self::Known(KnownResponseOutputItem::Message(message)) => Some(message),
            _ => None,
        }
    }

    /// 返回函数调用输出项。
    pub fn as_function_call(&self) -> Option<&ResponseFunctionToolCall> {
        match self {
            Self::Known(KnownResponseOutputItem::FunctionCall(call)) => Some(call),
            _ => None,
        }
    }

    /// 返回原始 JSON 项。
    pub fn as_raw(&self) -> Option<&Value> {
        match self {
            Self::Raw(value) => Some(value.as_raw()),
            _ => None,
        }
    }

    /// 提取输出文本。
    pub fn output_text(&self) -> Option<&str> {
        match self {
            Self::Known(KnownResponseOutputItem::OutputText(text)) => Some(text.text.as_str()),
            Self::Known(KnownResponseOutputItem::Message(message)) => message
                .content
                .iter()
                .find_map(ResponseOutputContentPart::text),
            Self::Raw(value) => {
                let value = value.as_raw();
                if let Some(text) = value.get("text").and_then(Value::as_str) {
                    return Some(text);
                }
                value
                    .get("content")
                    .and_then(Value::as_array)
                    .and_then(|content| {
                        content
                            .iter()
                            .find_map(|item| item.get("text").and_then(Value::as_str))
                    })
            }
            _ => None,
        }
    }
}

/// 已知的响应输出项类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KnownResponseOutputItem {
    /// assistant message。
    Message(ResponseOutputMessage),
    /// function call。
    FunctionCall(ResponseFunctionToolCall),
    /// 某些兼容 Provider 直接把 `output_text` 作为顶层输出项返回。
    OutputText(ResponseOutputText),
    /// 某些兼容 Provider 直接把 `refusal` 作为顶层输出项返回。
    Refusal(ResponseOutputRefusal),
}

/// 表示 assistant message 输出。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseOutputMessage {
    /// message ID。
    pub id: String,
    /// 内容片段。
    #[serde(default)]
    pub content: Vec<ResponseOutputContentPart>,
    /// 角色。
    pub role: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// assistant phase。
    pub phase: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示函数工具调用输出项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseFunctionToolCall {
    /// 输出项 ID。
    pub id: String,
    /// tool call ID。
    pub call_id: Option<String>,
    /// 工具名称。
    pub name: Option<String>,
    /// 参数 JSON 字符串。
    #[serde(default)]
    pub arguments: String,
    /// 状态。
    pub status: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 message 中的内容片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseOutputContentPart {
    /// 已知内容片段。
    Known(KnownResponseOutputContentPart),
    /// 向前兼容保留的原始片段。
    Raw(ResponseOutputContentPartRaw),
}

impl Default for ResponseOutputContentPart {
    fn default() -> Self {
        Self::Raw(ResponseOutputContentPartRaw::default())
    }
}

impl ResponseOutputContentPart {
    /// 返回 output_text 内容片段。
    pub fn as_output_text(&self) -> Option<&ResponseOutputText> {
        match self {
            Self::Known(KnownResponseOutputContentPart::OutputText(text)) => Some(text),
            _ => None,
        }
    }

    /// 提取文本内容。
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Known(KnownResponseOutputContentPart::OutputText(text)) => {
                Some(text.text.as_str())
            }
            Self::Raw(value) => value.as_raw().get("text").and_then(Value::as_str),
            _ => None,
        }
    }
}

/// 已知的 message 内容片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KnownResponseOutputContentPart {
    /// 输出文本。
    OutputText(ResponseOutputText),
    /// 拒绝回答。
    Refusal(ResponseOutputRefusal),
}

/// 表示 output_text 注解中的文件引用。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ResponseOutputTextFileCitation {
    /// 文件 ID。
    pub file_id: String,
    /// 文件名。
    pub filename: String,
    /// 文件索引。
    pub index: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 output_text 注解中的 URL 引用。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ResponseOutputTextUrlCitation {
    /// 引用结束位置。
    pub end_index: u64,
    /// 引用起始位置。
    pub start_index: u64,
    /// 标题。
    pub title: String,
    /// URL。
    pub url: String,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 output_text 注解中的容器文件引用。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ResponseOutputTextContainerFileCitation {
    /// 容器 ID。
    pub container_id: String,
    /// 引用结束位置。
    pub end_index: u64,
    /// 文件 ID。
    pub file_id: String,
    /// 文件名。
    pub filename: String,
    /// 引用起始位置。
    pub start_index: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 output_text 注解中的文件路径。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ResponseOutputTextFilePath {
    /// 文件 ID。
    pub file_id: String,
    /// 文件索引。
    pub index: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 已知的 output_text 注解类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KnownResponseOutputTextAnnotation {
    /// 文件引用。
    FileCitation(ResponseOutputTextFileCitation),
    /// URL 引用。
    UrlCitation(ResponseOutputTextUrlCitation),
    /// 容器文件引用。
    ContainerFileCitation(ResponseOutputTextContainerFileCitation),
    /// 文件路径。
    FilePath(ResponseOutputTextFilePath),
}

/// 向前兼容保留的未知 output_text 注解。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ResponseOutputTextAnnotationUnknown {
    /// 注解类型。
    #[serde(rename = "type")]
    pub annotation_type: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 output_text 注解。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ResponseOutputTextAnnotation {
    /// 已知注解。
    Known(KnownResponseOutputTextAnnotation),
    /// 未知注解。
    Unknown(ResponseOutputTextAnnotationUnknown),
}

impl Default for ResponseOutputTextAnnotation {
    fn default() -> Self {
        Self::Unknown(ResponseOutputTextAnnotationUnknown::default())
    }
}

impl ResponseOutputTextAnnotation {
    /// 返回注解类型。
    pub fn kind(&self) -> Option<&str> {
        match self {
            Self::Known(KnownResponseOutputTextAnnotation::FileCitation(_)) => {
                Some("file_citation")
            }
            Self::Known(KnownResponseOutputTextAnnotation::UrlCitation(_)) => Some("url_citation"),
            Self::Known(KnownResponseOutputTextAnnotation::ContainerFileCitation(_)) => {
                Some("container_file_citation")
            }
            Self::Known(KnownResponseOutputTextAnnotation::FilePath(_)) => Some("file_path"),
            Self::Unknown(annotation) => annotation.annotation_type.as_deref(),
        }
    }
}

/// 表示 output_text 的 top logprob。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ResponseOutputTextTopLogprob {
    /// token 内容。
    #[serde(default)]
    pub token: String,
    /// UTF-8 bytes。
    #[serde(default)]
    pub bytes: Vec<u8>,
    /// token 对应的 logprob。
    #[serde(default)]
    pub logprob: f64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 output_text 的单个 token logprob。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ResponseOutputTextLogprob {
    /// token 内容。
    #[serde(default)]
    pub token: String,
    /// UTF-8 bytes。
    #[serde(default)]
    pub bytes: Vec<u8>,
    /// token 对应的 logprob。
    #[serde(default)]
    pub logprob: f64,
    /// top logprobs。
    #[serde(default)]
    pub top_logprobs: Vec<ResponseOutputTextTopLogprob>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示输出文本片段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseOutputText {
    /// 注解。
    #[serde(default)]
    pub annotations: Vec<ResponseOutputTextAnnotation>,
    /// 文本。
    #[serde(default)]
    pub text: String,
    /// token 级 logprobs。
    pub logprobs: Option<Vec<ResponseOutputTextLogprob>>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示拒绝回答片段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseOutputRefusal {
    /// 拒绝原因。
    #[serde(default)]
    pub refusal: String,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 Responses 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseCreateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 输入载荷。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInputPayload>,
    /// 温度。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 工具定义。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ChatToolDefinition>,
    /// 是否启用流式。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

fn default_function_type() -> String {
    "function".into()
}

fn parse_jsonish_payload<T>(payload: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let trimmed = payload.trim();
    let normalized = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|value| value.trim())
        .and_then(|value| value.strip_suffix("```"))
        .map_or(trimmed, str::trim);
    serde_json::from_str(normalized).map_err(|error| {
        Error::Serialization(crate::SerializationError::new(format!(
            "结构化 JSON 解析失败: {error}"
        )))
    })
}

fn parse_json_arguments<T>(arguments: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(arguments).map_err(|error| {
        Error::Serialization(crate::SerializationError::new(format!(
            "工具参数 JSON 解析失败: {error}"
        )))
    })
}

handle!(
    /// 顶层 completions 资源。
    CompletionsResource
);
handle!(
    /// 聊天资源命名空间。
    ChatResource
);
handle!(
    /// 聊天补全资源。
    ChatCompletionsResource
);
handle!(
    /// 聊天补全消息子资源。
    ChatCompletionMessagesResource
);
handle!(
    /// Embeddings 资源。
    EmbeddingsResource
);
handle!(
    /// Files 资源。
    FilesResource
);
handle!(
    /// Images 资源。
    ImagesResource
);
handle!(
    /// Audio 资源命名空间。
    AudioResource
);
handle!(
    /// Audio Speech 资源。
    AudioSpeechResource
);
handle!(
    /// Audio Transcriptions 资源。
    AudioTranscriptionsResource
);
handle!(
    /// Audio Translations 资源。
    AudioTranslationsResource
);
handle!(
    /// Moderations 资源。
    ModerationsResource
);
handle!(
    /// Models 资源。
    ModelsResource
);
handle!(
    /// Fine-tuning 资源命名空间。
    FineTuningResource
);
handle!(
    /// Fine-tuning Jobs 资源。
    FineTuningJobsResource
);
handle!(
    /// Fine-tuning Checkpoints 资源。
    FineTuningJobCheckpointsResource
);
handle!(
    /// Fine-tuning 权限资源。
    FineTuningCheckpointPermissionsResource
);
handle!(
    /// Fine-tuning Alpha 命名空间。
    FineTuningAlphaResource
);
handle!(
    /// Fine-tuning Alpha Graders 资源。
    FineTuningAlphaGradersResource
);
handle!(
    /// Graders 资源命名空间。
    GradersResource
);
handle!(
    /// Vector Stores 资源。
    VectorStoresResource
);
handle!(
    /// Vector Store Files 资源。
    VectorStoreFilesResource
);
handle!(
    /// Vector Store File Batches 资源。
    VectorStoreFileBatchesResource
);
handle!(
    /// Batches 资源。
    BatchesResource
);
handle!(
    /// Uploads 资源。
    UploadsResource
);
handle!(
    /// Uploads Parts 资源。
    UploadPartsResource
);
handle!(
    /// Responses 资源。
    ResponsesResource
);
handle!(
    /// Responses Input Items 资源。
    ResponseInputItemsResource
);
handle!(
    /// Responses Input Tokens 资源。
    ResponseInputTokensResource
);
handle!(
    /// Realtime 资源命名空间。
    RealtimeResource
);
handle!(
    /// Realtime Client Secrets 资源。
    RealtimeClientSecretsResource
);
handle!(
    /// Realtime Calls 资源。
    RealtimeCallsResource
);
handle!(
    /// Conversations 资源。
    ConversationsResource
);
handle!(
    /// Conversation Items 资源。
    ConversationItemsResource
);
handle!(
    /// Evals 资源。
    EvalsResource
);
handle!(
    /// Eval Runs 资源。
    EvalRunsResource
);
handle!(
    /// Eval Run Output Items 资源。
    EvalRunOutputItemsResource
);
handle!(
    /// Containers 资源。
    ContainersResource
);
handle!(
    /// Container Files 资源。
    ContainerFilesResource
);
handle!(
    /// Container File Content 资源。
    ContainerFilesContentResource
);
handle!(
    /// Skills 资源。
    SkillsResource
);
handle!(
    /// Skills Content 资源。
    SkillsContentResource
);
handle!(
    /// Skills Versions 资源。
    SkillVersionsResource
);
handle!(
    /// Skills Versions Content 资源。
    SkillVersionsContentResource
);
handle!(
    /// Videos 资源。
    VideosResource
);
handle!(
    /// Webhooks 资源。
    WebhooksResource
);
handle!(
    /// Beta 资源命名空间。
    BetaResource
);
handle!(
    /// Beta Assistants 资源。
    BetaAssistantsResource
);
handle!(
    /// Beta Threads 资源。
    BetaThreadsResource
);
handle!(
    /// Beta Thread Messages 资源。
    BetaThreadMessagesResource
);
handle!(
    /// Beta Thread Runs 资源。
    BetaThreadRunsResource
);
handle!(
    /// Beta Thread Run Steps 资源。
    BetaThreadRunStepsResource
);
handle!(
    /// Beta ChatKit 命名空间。
    BetaChatkitResource
);
handle!(
    /// Beta ChatKit Sessions 资源。
    BetaChatkitSessionsResource
);
handle!(
    /// Beta ChatKit Threads 资源。
    BetaChatkitThreadsResource
);
handle!(
    /// Beta Realtime 命名空间。
    BetaRealtimeResource
);
handle!(
    /// Beta Realtime Sessions 资源。
    BetaRealtimeSessionsResource
);
handle!(
    /// Beta Realtime Transcription Sessions 资源。
    BetaRealtimeTranscriptionSessionsResource
);

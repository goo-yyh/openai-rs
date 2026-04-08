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

pub use beta::{
    BetaAssistant, BetaRealtimeSession, BetaRealtimeTranscriptionSession, BetaThread,
    BetaThreadMessage, BetaThreadRun, BetaThreadRunStep, ChatKitSession, ChatKitThread,
    ChatKitThreadItem, ChatKitThreadStatus,
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
    Completion, CompletionChoice, CompletionLogProbs, CompletionUsage, ModerationCreateResponse,
    ModerationResult,
};
pub use fine_tuning::{
    GraderModel, GraderModelCatalog, GraderRunErrors, GraderRunMetadata, GraderRunResponse,
    GraderValidateResponse,
};
pub use longtail::{
    AudioSpeechCreateParams, AudioSpeechRequestBuilder, AudioTranscription,
    AudioTranscriptionRequestBuilder, AudioTranslation, AudioTranslationRequestBuilder, Batch,
    BatchCreateParams, BatchCreateRequestBuilder, Container, ContainerCreateParams, ContainerFile,
    ContainerFileCreateParams, Conversation, ConversationCreateParams, ConversationItem,
    ConversationItemCreateParams, ConversationUpdateParams, Eval, EvalCreateParams, EvalOutputItem,
    EvalRun, EvalRunCreateParams, EvalUpdateParams, FineTuningCheckpoint,
    FineTuningCheckpointPermission, FineTuningJob, FineTuningJobCreateParams,
    FineTuningJobCreateRequestBuilder, FineTuningJobEvent, ImageData, ImageGenerateParams,
    ImageGenerateRequestBuilder, ImageGenerationResponse, Skill, SkillCreateParams,
    SkillUpdateParams, SkillVersion, SkillVersionCreateParams, Video, VideoCharacter,
    VideoCharacterCreateParams, VideoCreateParams,
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
    pub data: Vec<Value>,
    /// 使用统计。
    pub usage: Option<Value>,
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
    pub reasoning_details: Vec<Value>,
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
    pub logprobs: Option<Value>,
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
    pub usage: Option<Value>,
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
    pub reasoning_details: Vec<Value>,
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
    pub logprobs: Option<Value>,
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
    pub values: Vec<Value>,
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
                .and_then(|logprobs| logprobs.get(field_name))
                .and_then(Value::as_array)?
                .clone();
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
    pub parameters: Value,
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
    pub tool_choice: Option<Value>,
    /// 流式开关。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// 表示 Responses 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Response {
    /// 响应 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 模型 ID。
    pub model: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 输出项。
    #[serde(default)]
    pub output: Vec<Value>,
    /// 用量统计。
    pub usage: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Response {
    /// 尝试提取最终文本输出。
    pub fn output_text(&self) -> Option<String> {
        for item in &self.output {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                return Some(text.to_owned());
            }
            if let Some(content) = item.get("content").and_then(Value::as_array) {
                for content_item in content {
                    if let Some(text) = content_item.get("text").and_then(Value::as_str) {
                        return Some(text.to_owned());
                    }
                }
            }
        }

        self.extra
            .get("output_text")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

/// 表示 Responses 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseCreateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 输入载荷。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
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

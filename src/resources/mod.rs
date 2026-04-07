//! 资源命名空间、公开类型与请求构建器。

mod beta;
mod chat;
mod longtail;
mod responses;
mod vector_stores;

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::time::Duration;

use bytes::Bytes;
use http::Method;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::Client;
use crate::config::RequestOptions;
use crate::error::{Error, Result};
use crate::files::{MultipartField, UploadSource};
#[cfg(feature = "structured-output")]
use crate::helpers::{ParsedChatCompletion, ParsedResponse, parse_json_payload};
#[cfg(feature = "tool-runner")]
use crate::helpers::{ToolDefinition, ToolRegistry};
use crate::pagination::{CursorPage, ListEnvelope};
use crate::response_meta::ApiResponse;
#[cfg(feature = "tool-runner")]
use crate::stream::ChatCompletionRuntimeEvent;
use crate::stream::{
    AssistantEventStream, AssistantStream, ChatCompletionEventStream, ChatCompletionStream,
    RawSseStream, ResponseEventStream, ResponseStream, SseStream,
};
use crate::transport::{RequestSpec, merge_json_body};
use crate::webhooks::{HeaderLookup, WebhookVerifier};
#[cfg(feature = "realtime")]
use crate::websocket::RealtimeSocket;
#[cfg(feature = "responses-ws")]
use crate::websocket::ResponsesSocket;
#[cfg(feature = "tool-runner")]
use futures_util::StreamExt;

pub use beta::{BetaAssistant, BetaThread, BetaThreadMessage, BetaThreadRun, BetaThreadRunStep};
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
pub use vector_stores::{
    VectorStore, VectorStoreFile, VectorStoreFileBatch, VectorStoreSearchResponse,
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

const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b'!')
    .add(b'$')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'}');

fn value_from<T>(value: &T) -> Result<Value>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .map_err(|error| Error::Serialization(crate::SerializationError::new(error.to_string())))
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

/// 对单个路径参数做安全编码，避免动态 ID 改写 URL 结构。
pub(crate) fn encode_path_segment(segment: impl AsRef<str>) -> String {
    utf8_percent_encode(segment.as_ref(), PATH_SEGMENT_ENCODE_SET).to_string()
}

/// 表示通用 JSON 请求构建器。
#[derive(Debug, Clone)]
pub struct JsonRequestBuilder<T> {
    client: Client,
    spec: RequestSpec,
    extra_body: BTreeMap<String, Value>,
    provider_options: BTreeMap<String, Value>,
    _marker: PhantomData<T>,
}

impl<T> JsonRequestBuilder<T> {
    fn new(
        client: Client,
        endpoint_id: &'static str,
        method: Method,
        path: impl Into<String>,
    ) -> Self {
        Self {
            client,
            spec: RequestSpec::new(endpoint_id, method, path),
            extra_body: BTreeMap::new(),
            provider_options: BTreeMap::new(),
            _marker: PhantomData,
        }
    }

    /// 设置整个请求体为一个 `serde_json::Value`。
    pub fn body_value(mut self, body: Value) -> Self {
        self.spec.body = Some(body);
        self
    }

    /// 使用任意可序列化对象设置请求体。
    ///
    /// # Errors
    ///
    /// 当序列化失败时返回错误。
    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.spec.body = Some(value_from(body)?);
        Ok(self)
    }

    /// 添加一个额外请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.spec.options.insert_header(key, value);
        self
    }

    /// 删除一个默认请求头。
    pub fn remove_header(mut self, key: impl Into<String>) -> Self {
        self.spec.options.remove_header(key);
        self
    }

    /// 添加一个额外查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.spec.options.insert_query(key, value);
        self
    }

    /// 在 JSON 根对象中追加字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra_body.insert(key.into(), value);
        self
    }

    /// 在 provider 对应的 `provider_options` 下追加字段。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.provider_options.insert(key.into(), value);
        self
    }

    /// 覆盖请求超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.spec.options.timeout = Some(timeout);
        self
    }

    /// 覆盖最大重试次数。
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.spec.options.max_retries = Some(max_retries);
        self
    }

    /// 设置取消令牌。
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.spec.options.cancellation_token = Some(token);
        self
    }

    /// 添加 Multipart 文本字段。
    pub fn multipart_text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let multipart = self.spec.multipart.get_or_insert_default();
        multipart.fields.push(MultipartField {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// 添加 Multipart 文件字段。
    pub fn multipart_file(mut self, name: impl Into<String>, file: UploadSource) -> Self {
        let multipart = self.spec.multipart.get_or_insert_default();
        multipart.files.push((name.into(), file));
        self
    }

    fn into_spec(mut self) -> RequestSpec {
        let provider_key = self.client.provider().kind().as_key();
        self.spec.body = Some(merge_json_body(
            self.spec.body.take(),
            &self.extra_body,
            provider_key,
            &self.provider_options,
        ));
        self.spec
    }
}

impl<T> JsonRequestBuilder<T>
where
    T: serde::de::DeserializeOwned,
{
    /// 发送请求并返回业务对象。
    ///
    /// # Errors
    ///
    /// 当请求失败或反序列化失败时返回错误。
    pub async fn send(self) -> Result<T> {
        Ok(self.send_with_meta().await?.data)
    }

    /// 发送请求并保留响应元信息。
    ///
    /// # Errors
    ///
    /// 当请求失败或反序列化失败时返回错误。
    pub async fn send_with_meta(self) -> Result<ApiResponse<T>> {
        let client = self.client.clone();
        client.execute_json(self.into_spec()).await
    }

    /// 发送请求并返回原始 `http::Response<Bytes>`。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        let client = self.client.clone();
        client.execute_raw_http(self.into_spec()).await
    }

    /// 发送请求并返回原始 SSE 事件流。
    ///
    /// 该方法会自动追加 `Accept: text/event-stream` 请求头。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send_raw_sse(self) -> Result<RawSseStream> {
        let client = self.client.clone();
        let mut spec = self.into_spec();
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_raw_sse(spec).await
    }
}

impl<T> JsonRequestBuilder<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    /// 发送请求并把 SSE 数据流解析为指定类型。
    ///
    /// 该方法会自动追加 `Accept: text/event-stream` 请求头。
    ///
    /// # Errors
    ///
    /// 当请求失败或 SSE 事件反序列化失败时返回错误。
    pub async fn send_sse(self) -> Result<SseStream<T>> {
        let client = self.client.clone();
        let mut spec = self.into_spec();
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_sse(spec).await
    }
}

/// 表示二进制响应请求构建器。
#[derive(Debug, Clone)]
pub struct BytesRequestBuilder {
    inner: JsonRequestBuilder<Bytes>,
}

impl BytesRequestBuilder {
    fn new(
        client: Client,
        endpoint_id: &'static str,
        method: Method,
        path: impl Into<String>,
    ) -> Self {
        Self {
            inner: JsonRequestBuilder::new(client, endpoint_id, method, path),
        }
    }

    /// 设置 JSON 请求体。
    pub fn body_value(mut self, body: Value) -> Self {
        self.inner = self.inner.body_value(body);
        self
    }

    /// 设置可序列化请求体。
    ///
    /// # Errors
    ///
    /// 当序列化失败时返回错误。
    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.inner = self.inner.json_body(body)?;
        Ok(self)
    }

    /// 追加请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_header(key, value);
        self
    }

    /// 追加查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query(key, value);
        self
    }

    /// 在 provider 对应的 `provider_options` 下追加字段。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    /// 覆盖请求超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// 覆盖最大重试次数。
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.inner = self.inner.max_retries(max_retries);
        self
    }

    /// 设置取消令牌。
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.inner = self.inner.cancellation_token(token);
        self
    }

    /// 添加 Multipart 文本字段。
    pub fn multipart_text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text(name, value);
        self
    }

    /// 添加 Multipart 文件字段。
    pub fn multipart_file(mut self, name: impl Into<String>, file: UploadSource) -> Self {
        self.inner = self.inner.multipart_file(name, file);
        self
    }

    /// 在 JSON 根对象中追加字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    /// 发送请求并返回原始字节。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send(self) -> Result<Bytes> {
        Ok(self.send_with_meta().await?.data)
    }

    /// 发送请求并保留响应元信息。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send_with_meta(self) -> Result<ApiResponse<Bytes>> {
        let client = self.inner.client.clone();
        client.execute_bytes(self.inner.into_spec()).await
    }

    /// 发送请求并返回原始 HTTP 响应。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        let client = self.inner.client.clone();
        client.execute_raw_http(self.inner.into_spec()).await
    }

    /// 发送请求并返回原始 SSE 事件流。
    ///
    /// 该方法会自动追加 `Accept: text/event-stream` 请求头。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send_raw_sse(self) -> Result<RawSseStream> {
        let client = self.inner.client.clone();
        let mut spec = self.inner.into_spec();
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_raw_sse(spec).await
    }

    /// 发送请求并把 SSE 数据流解析为指定类型。
    ///
    /// 该方法会自动追加 `Accept: text/event-stream` 请求头。
    ///
    /// # Errors
    ///
    /// 当请求失败或 SSE 事件反序列化失败时返回错误。
    pub async fn send_sse<T>(self) -> Result<SseStream<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let client = self.inner.client.clone();
        let mut spec = self.inner.into_spec();
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_sse(spec).await
    }
}

/// 表示列表请求构建器。
#[derive(Debug, Clone)]
pub struct ListRequestBuilder<T> {
    inner: JsonRequestBuilder<ListEnvelope<T>>,
}

impl<T> ListRequestBuilder<T> {
    fn new(client: Client, endpoint_id: &'static str, path: impl Into<String>) -> Self {
        Self {
            inner: JsonRequestBuilder::new(client, endpoint_id, Method::GET, path),
        }
    }

    /// 设置 `after` 游标。
    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query("after", cursor);
        self
    }

    /// 设置 `before` 游标。
    pub fn before(mut self, cursor: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query("before", cursor);
        self
    }

    /// 设置分页大小。
    pub fn limit(mut self, limit: u32) -> Self {
        self.inner = self.inner.extra_query("limit", limit.to_string());
        self
    }

    /// 追加请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_header(key, value);
        self
    }

    /// 在根对象追加额外字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }
}

impl<T> ListRequestBuilder<T>
where
    T: Clone + Send + Sync + serde::de::DeserializeOwned + 'static,
{
    /// 发送列表请求并返回游标分页对象。
    ///
    /// # Errors
    ///
    /// 当请求失败或反序列化失败时返回错误。
    pub async fn send(self) -> Result<CursorPage<T>> {
        let client = self.inner.client.clone();
        let path = self.inner.spec.path.clone();
        let endpoint_id = self.inner.spec.endpoint_id;
        let response = client
            .execute_json::<ListEnvelope<T>>(self.inner.into_spec())
            .await?;
        let ListEnvelope {
            object,
            data,
            first_id,
            last_id,
            has_more,
            extra,
        } = response.data;
        let mut next_query = BTreeMap::new();
        if let Some(last_id) = &last_id {
            next_query.insert("after".into(), Some(last_id.clone()));
        }
        Ok(CursorPage::from(ListEnvelope {
            object,
            data,
            first_id,
            last_id,
            has_more,
            extra,
        })
        .with_next_request(if has_more {
            Some(crate::client::PageRequestSpec {
                client,
                endpoint_id,
                method: Method::GET,
                path,
                query: next_query,
            })
        } else {
            None
        }))
    }
}

/// 表示聊天补全创建构建器。
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionCreateRequestBuilder {
    client: Option<Client>,
    params: ChatCompletionCreateParams,
    options: RequestOptions,
    extra_body: BTreeMap<String, Value>,
    provider_options: BTreeMap<String, Value>,
}

impl ChatCompletionCreateRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            client: Some(client),
            ..Self::default()
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.params.model = Some(model.into());
        self
    }

    /// 直接设置消息列表。
    pub fn messages(mut self, messages: Vec<ChatCompletionMessage>) -> Self {
        self.params.messages = messages;
        self
    }

    /// 追加一条 system 消息。
    pub fn message_system(mut self, content: impl Into<String>) -> Self {
        self.params
            .messages
            .push(ChatCompletionMessage::system(content));
        self
    }

    /// 追加一条 user 消息。
    pub fn message_user(mut self, content: impl Into<String>) -> Self {
        self.params
            .messages
            .push(ChatCompletionMessage::user(content));
        self
    }

    /// 追加一条 assistant 消息。
    pub fn message_assistant(mut self, content: impl Into<String>) -> Self {
        self.params
            .messages
            .push(ChatCompletionMessage::assistant(content));
        self
    }

    /// 设置温度。
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.params.temperature = Some(temperature);
        self
    }

    /// 设置候选数量。
    pub fn n(mut self, n: u32) -> Self {
        self.params.n = Some(n);
        self
    }

    /// 设置最大 token 数。
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.params.max_tokens = Some(max_tokens);
        self
    }

    /// 追加工具定义。
    pub fn tool(mut self, tool: ChatToolDefinition) -> Self {
        self.params.tools.push(tool);
        self
    }

    /// 设置工具选择策略。
    pub fn tool_choice(mut self, tool_choice: Value) -> Self {
        self.params.tool_choice = Some(tool_choice);
        self
    }

    /// 设置附加请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_header(key, value);
        self
    }

    /// 设置附加查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_query(key, value);
        self
    }

    /// 在请求体根对象中追加字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra_body.insert(key.into(), value);
        self
    }

    /// 在 provider 对应的 `provider_options` 节点下追加字段。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.provider_options.insert(key.into(), value);
        self
    }

    /// 覆盖超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    /// 设置取消令牌。
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.options.cancellation_token = Some(token);
        self
    }

    fn build_spec(mut self, stream: bool) -> Result<(Client, RequestSpec)> {
        let client = self
            .client
            .take()
            .ok_or_else(|| Error::InvalidConfig("聊天补全构建器缺少客户端".into()))?;
        if self.params.model.as_deref().unwrap_or_default().is_empty() {
            return Err(Error::MissingRequiredField { field: "model" });
        }
        if self.params.messages.is_empty() {
            return Err(Error::MissingRequiredField { field: "messages" });
        }
        self.params.stream = Some(stream);
        let provider_key = client.provider().kind().as_key();
        let body = merge_json_body(
            Some(value_from(&self.params)?),
            &self.extra_body,
            provider_key,
            &self.provider_options,
        );
        let mut spec = RequestSpec::new(
            if stream {
                "chat.completions.stream"
            } else {
                "chat.completions.create"
            },
            Method::POST,
            "/chat/completions",
        );
        spec.body = Some(body);
        spec.options = self.options;
        Ok((client, spec))
    }

    /// 发送普通聊天补全请求。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或反序列化失败时返回错误。
    pub async fn send(self) -> Result<ChatCompletion> {
        Ok(self.send_with_meta().await?.data)
    }

    /// 发送普通聊天补全请求并保留元信息。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或反序列化失败时返回错误。
    pub async fn send_with_meta(self) -> Result<ApiResponse<ChatCompletion>> {
        let (client, spec) = self.build_spec(false)?;
        client.execute_json(spec).await
    }

    /// 发送普通聊天补全请求并返回原始 HTTP 响应。
    ///
    /// # Errors
    ///
    /// 当参数校验失败或请求失败时返回错误。
    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        let (client, spec) = self.build_spec(false)?;
        client.execute_raw_http(spec).await
    }
}

/// 表示聊天补全流式请求构建器。
#[derive(Debug, Clone)]
pub struct ChatCompletionStreamRequestBuilder {
    inner: ChatCompletionCreateRequestBuilder,
}

/// 表示 Assistants/Beta Threads 流式请求构建器。
#[derive(Debug, Clone)]
pub struct AssistantStreamRequestBuilder {
    inner: JsonRequestBuilder<Value>,
}

impl ChatCompletionStreamRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            inner: ChatCompletionCreateRequestBuilder::new(client),
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.model(model);
        self
    }

    /// 设置消息列表。
    pub fn messages(mut self, messages: Vec<ChatCompletionMessage>) -> Self {
        self.inner = self.inner.messages(messages);
        self
    }

    /// 追加一条 system 消息。
    pub fn message_system(mut self, content: impl Into<String>) -> Self {
        self.inner = self.inner.message_system(content);
        self
    }

    /// 追加一条 user 消息。
    pub fn message_user(mut self, content: impl Into<String>) -> Self {
        self.inner = self.inner.message_user(content);
        self
    }

    /// 追加一条 assistant 消息。
    pub fn message_assistant(mut self, content: impl Into<String>) -> Self {
        self.inner = self.inner.message_assistant(content);
        self
    }

    /// 设置温度。
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.inner = self.inner.temperature(temperature);
        self
    }

    /// 设置候选数量。
    pub fn n(mut self, n: u32) -> Self {
        self.inner = self.inner.n(n);
        self
    }

    /// 设置最大 token 数。
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.inner = self.inner.max_tokens(max_tokens);
        self
    }

    /// 添加额外字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    /// 添加 provider 选项。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    /// 发送流式聊天补全请求。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或流初始化失败时返回错误。
    pub async fn send(self) -> Result<ChatCompletionStream> {
        let (client, spec) = self.inner.build_spec(true)?;
        Ok(ChatCompletionStream::new(client.execute_sse(spec).await?))
    }

    /// 发送流式聊天补全请求，并返回带高层语义事件的运行时流。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或流初始化失败时返回错误。
    pub async fn send_events(self) -> Result<ChatCompletionEventStream> {
        Ok(self.send().await?.events())
    }
}

impl AssistantStreamRequestBuilder {
    fn new(
        client: Client,
        endpoint_id: &'static str,
        method: Method,
        path: impl Into<String>,
    ) -> Self {
        Self {
            inner: JsonRequestBuilder::new(client, endpoint_id, method, path),
        }
    }

    /// 设置整个请求体为一个 `serde_json::Value`。
    pub fn body_value(mut self, body: Value) -> Self {
        self.inner = self.inner.body_value(body);
        self
    }

    /// 使用任意可序列化对象设置请求体。
    ///
    /// # Errors
    ///
    /// 当序列化失败时返回错误。
    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.inner = self.inner.json_body(body)?;
        Ok(self)
    }

    /// 添加一个额外请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_header(key, value);
        self
    }

    /// 删除一个默认请求头。
    pub fn remove_header(mut self, key: impl Into<String>) -> Self {
        self.inner = self.inner.remove_header(key);
        self
    }

    /// 添加一个额外查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query(key, value);
        self
    }

    /// 在 JSON 根对象中追加字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    /// 在 provider 对应的 `provider_options` 下追加字段。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    /// 覆盖请求超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// 覆盖最大重试次数。
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.inner = self.inner.max_retries(max_retries);
        self
    }

    /// 设置取消令牌。
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.inner = self.inner.cancellation_token(token);
        self
    }

    /// 发送流式 Assistants 请求。
    ///
    /// # Errors
    ///
    /// 当请求失败或流初始化失败时返回错误。
    pub async fn send(self) -> Result<AssistantStream> {
        let client = self.inner.client.clone();
        let stream = client.execute_raw_sse(self.inner.into_spec()).await?;
        Ok(AssistantStream::new(stream))
    }

    /// 发送流式 Assistants 请求，并返回带高层派生事件的运行时流。
    ///
    /// # Errors
    ///
    /// 当请求失败或流初始化失败时返回错误。
    pub async fn send_events(self) -> Result<AssistantEventStream> {
        Ok(self.send().await?.events())
    }
}

/// 表示聊天补全结构化解析构建器。
#[cfg(feature = "structured-output")]
#[derive(Debug, Clone)]
pub struct ChatCompletionParseRequestBuilder<T> {
    inner: ChatCompletionCreateRequestBuilder,
    _marker: PhantomData<T>,
}

#[cfg(feature = "structured-output")]
impl<T> ChatCompletionParseRequestBuilder<T> {
    fn new(client: Client) -> Self {
        Self {
            inner: ChatCompletionCreateRequestBuilder::new(client),
            _marker: PhantomData,
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.model(model);
        self
    }

    /// 设置消息列表。
    pub fn messages(mut self, messages: Vec<ChatCompletionMessage>) -> Self {
        self.inner = self.inner.messages(messages);
        self
    }

    /// 追加一条 user 消息。
    pub fn message_user(mut self, content: impl Into<String>) -> Self {
        self.inner = self.inner.message_user(content);
        self
    }

    /// 追加一个额外请求体字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }
}

#[cfg(feature = "structured-output")]
impl<T> ChatCompletionParseRequestBuilder<T>
where
    T: JsonSchema + serde::de::DeserializeOwned,
{
    /// 发送请求并把首条 assistant 文本解析成结构化对象。
    ///
    /// # Errors
    ///
    /// 当模型返回内容为空或 JSON 解析失败时返回错误。
    pub async fn send(self) -> Result<ParsedChatCompletion<T>> {
        let response = self.inner.send().await?;
        response.ensure_not_truncated()?;
        let choice = response
            .choices
            .first()
            .ok_or_else(|| Error::InvalidConfig("聊天补全返回中缺少 choice".into()))?;
        let parsed = if let Some(content) = choice.message.content.as_deref() {
            parse_json_payload(content)?
        } else if let Some(parsed_arguments) = choice.message.parse_tool_arguments()? {
            parsed_arguments
        } else {
            return Err(Error::InvalidConfig(
                "聊天补全返回中既没有 assistant 文本，也没有可解析的工具参数".into(),
            ));
        };
        Ok(ParsedChatCompletion { response, parsed })
    }
}

/// 表示工具运行构建器。
#[cfg(feature = "tool-runner")]
#[derive(Debug, Clone)]
pub struct ChatCompletionRunToolsRequestBuilder {
    inner: ChatCompletionCreateRequestBuilder,
    registry: ToolRegistry,
    max_rounds: usize,
}

#[cfg(feature = "tool-runner")]
impl ChatCompletionRunToolsRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            inner: ChatCompletionCreateRequestBuilder::new(client),
            registry: ToolRegistry::new(),
            max_rounds: 8,
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.model(model);
        self
    }

    /// 设置消息列表。
    pub fn messages(mut self, messages: Vec<ChatCompletionMessage>) -> Self {
        self.inner = self.inner.messages(messages);
        self
    }

    /// 追加一条 user 消息。
    pub fn message_user(mut self, content: impl Into<String>) -> Self {
        self.inner = self.inner.message_user(content);
        self
    }

    /// 注册一个工具。
    pub fn register_tool(mut self, tool: ToolDefinition) -> Self {
        self.registry.register(tool);
        self
    }

    /// 设置最大工具轮次。
    pub fn max_rounds(mut self, max_rounds: usize) -> Self {
        self.max_rounds = max_rounds;
        self
    }

    /// 发送请求并返回工具调用运行 trace。
    ///
    /// # Errors
    ///
    /// 当工具不存在、工具执行失败或请求失败时返回错误。
    pub async fn into_runner(self) -> Result<ChatCompletionRunner> {
        let execution = self.execute(false).await?;
        Ok(ChatCompletionRunner::from_execution(execution))
    }

    /// 使用流式请求执行工具调用，并返回包含流式事件的运行 trace。
    ///
    /// # Errors
    ///
    /// 当工具不存在、工具执行失败、流式请求失败或结束原因不可解析时返回错误。
    pub async fn into_streaming_runner(self) -> Result<ChatCompletionStreamingRunner> {
        let execution = self.execute(true).await?;
        Ok(ChatCompletionStreamingRunner::from_execution(execution))
    }

    /// 发送请求并自动处理工具调用。
    ///
    /// # Errors
    ///
    /// 当工具不存在、工具执行失败或请求失败时返回错误。
    pub async fn send(self) -> Result<ChatCompletion> {
        self.into_runner()
            .await?
            .final_chat_completion()
            .cloned()
            .ok_or_else(|| Error::InvalidConfig("工具运行未返回最终聊天补全结果".into()))
    }

    /// 使用流式请求执行工具调用轮次，并返回最终聊天补全结果。
    ///
    /// # Errors
    ///
    /// 当工具不存在、工具执行失败、流式请求失败或结束原因不可解析时返回错误。
    pub async fn send_streaming(self) -> Result<ChatCompletion> {
        self.into_streaming_runner()
            .await?
            .final_chat_completion()
            .cloned()
            .ok_or_else(|| Error::InvalidConfig("工具运行未返回最终聊天补全结果".into()))
    }

    async fn execute(self, stream: bool) -> Result<ChatCompletionRunExecution> {
        let mut inner = self.inner;
        if inner.params.tools.is_empty() {
            inner.params.tools = self
                .registry
                .all()
                .map(ChatToolDefinition::from_tool)
                .collect();
        }

        let mut messages = inner.params.messages.clone();
        let mut execution = ChatCompletionRunExecution {
            messages: messages.clone(),
            ..ChatCompletionRunExecution::default()
        };
        for _ in 0..self.max_rounds {
            let request = ChatCompletionCreateRequestBuilder {
                params: ChatCompletionCreateParams {
                    messages: messages.clone(),
                    ..inner.params.clone()
                },
                ..inner.clone()
            };
            let response = if stream {
                let (client, spec) = request.build_spec(true)?;
                let mut event_stream =
                    ChatCompletionStream::new(client.execute_sse(spec).await?).events();
                while let Some(event) = event_stream.next().await {
                    execution.stream_events.push(event?);
                }
                let response = event_stream
                    .snapshot()
                    .ok_or_else(|| Error::InvalidConfig("流式聊天补全未返回最终结果".into()))?;
                response.ensure_not_truncated()?;
                response
            } else {
                request.send().await?
            };
            execution.chat_completions.push(response.clone());
            let Some(choice) = response.choices.first() else {
                return Ok(execution);
            };
            response.ensure_not_truncated()?;
            execution.messages.push(choice.message.clone());
            if choice.message.tool_calls.is_empty() {
                return Ok(execution);
            }

            messages.push(choice.message.clone());

            for tool_call in &choice.message.tool_calls {
                let tool = self.registry.get(&tool_call.function.name).ok_or_else(|| {
                    Error::InvalidConfig(format!("未注册工具: {}", tool_call.function.name))
                })?;
                let arguments = if tool_call.function.arguments.trim().is_empty() {
                    Value::Object(Default::default())
                } else {
                    serde_json::from_str(&tool_call.function.arguments)
                        .unwrap_or_else(|_| Value::String(tool_call.function.arguments.clone()))
                };
                let output = tool.invoke(arguments).await?;
                let content = if output.is_string() {
                    output.as_str().unwrap_or_default().to_owned()
                } else {
                    output.to_string()
                };
                let tool_message =
                    ChatCompletionMessage::tool(tool_call.id.clone(), content.clone());
                messages.push(tool_message.clone());
                execution.messages.push(tool_message);
                execution.tool_results.push(ChatCompletionToolResult {
                    tool_call: tool_call.clone(),
                    output: content,
                });
            }
        }

        Err(Error::InvalidConfig("工具调用轮次已超过上限".into()))
    }
}

/// 表示一次工具调用返回的结果。
#[cfg(feature = "tool-runner")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionToolResult {
    /// 对应的工具调用。
    pub tool_call: ChatCompletionToolCall,
    /// 工具执行输出文本。
    pub output: String,
}

#[cfg(feature = "tool-runner")]
#[derive(Debug, Clone, Default)]
struct ChatCompletionRunExecution {
    messages: Vec<ChatCompletionMessage>,
    chat_completions: Vec<ChatCompletion>,
    tool_results: Vec<ChatCompletionToolResult>,
    stream_events: Vec<ChatCompletionRuntimeEvent>,
}

/// 表示非流式工具调用运行 trace。
#[cfg(feature = "tool-runner")]
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionRunner {
    messages: Vec<ChatCompletionMessage>,
    chat_completions: Vec<ChatCompletion>,
    tool_results: Vec<ChatCompletionToolResult>,
}

#[cfg(feature = "tool-runner")]
impl ChatCompletionRunner {
    fn from_execution(execution: ChatCompletionRunExecution) -> Self {
        Self {
            messages: execution.messages,
            chat_completions: execution.chat_completions,
            tool_results: execution.tool_results,
        }
    }

    /// 返回运行过程中累积的全部消息。
    pub fn messages(&self) -> &[ChatCompletionMessage] {
        &self.messages
    }

    /// 返回运行过程中累积的全部聊天补全。
    pub fn all_chat_completions(&self) -> &[ChatCompletion] {
        &self.chat_completions
    }

    /// 返回运行过程中累积的全部工具调用结果。
    pub fn tool_results(&self) -> &[ChatCompletionToolResult] {
        &self.tool_results
    }

    /// 返回最终聊天补全结果。
    pub fn final_chat_completion(&self) -> Option<&ChatCompletion> {
        self.chat_completions.last()
    }

    /// 返回最终 assistant 消息。
    pub fn final_message(&self) -> Option<&ChatCompletionMessage> {
        self.messages
            .iter()
            .rev()
            .find(|message| message.role == "assistant")
    }

    /// 返回最终 assistant 文本。
    pub fn final_content(&self) -> Option<&str> {
        self.final_message()
            .and_then(|message| message.content.as_deref())
    }

    /// 返回最终工具调用。
    pub fn final_function_tool_call(&self) -> Option<&ChatCompletionToolCall> {
        self.tool_results.last().map(|result| &result.tool_call)
    }

    /// 返回最终工具调用结果。
    pub fn final_function_tool_call_result(&self) -> Option<&str> {
        self.tool_results
            .last()
            .map(|result| result.output.as_str())
    }

    /// 汇总所有补全响应中的 usage 字段。
    pub fn total_usage(&self) -> Option<Value> {
        let mut completion_tokens = 0u64;
        let mut prompt_tokens = 0u64;
        let mut total_tokens = 0u64;
        let mut found = false;
        for completion in &self.chat_completions {
            let Some(usage) = completion.usage.as_ref() else {
                continue;
            };
            completion_tokens += usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            prompt_tokens += usage
                .get("prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            total_tokens += usage
                .get("total_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            found = true;
        }

        found.then(|| {
            serde_json::json!({
                "completion_tokens": completion_tokens,
                "prompt_tokens": prompt_tokens,
                "total_tokens": total_tokens,
            })
        })
    }
}

/// 表示流式工具调用运行 trace。
#[cfg(feature = "tool-runner")]
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionStreamingRunner {
    runner: ChatCompletionRunner,
    stream_events: Vec<ChatCompletionRuntimeEvent>,
}

#[cfg(feature = "tool-runner")]
impl ChatCompletionStreamingRunner {
    fn from_execution(execution: ChatCompletionRunExecution) -> Self {
        Self {
            runner: ChatCompletionRunner::from_execution(ChatCompletionRunExecution {
                messages: execution.messages,
                chat_completions: execution.chat_completions,
                tool_results: execution.tool_results,
                stream_events: Vec::new(),
            }),
            stream_events: execution.stream_events,
        }
    }

    /// 返回流式运行过程中产生的全部高层事件。
    pub fn events(&self) -> &[ChatCompletionRuntimeEvent] {
        &self.stream_events
    }

    /// 返回运行过程中累积的全部消息。
    pub fn messages(&self) -> &[ChatCompletionMessage] {
        self.runner.messages()
    }

    /// 返回运行过程中累积的全部聊天补全。
    pub fn all_chat_completions(&self) -> &[ChatCompletion] {
        self.runner.all_chat_completions()
    }

    /// 返回运行过程中累积的全部工具调用结果。
    pub fn tool_results(&self) -> &[ChatCompletionToolResult] {
        self.runner.tool_results()
    }

    /// 返回最终聊天补全结果。
    pub fn final_chat_completion(&self) -> Option<&ChatCompletion> {
        self.runner.final_chat_completion()
    }

    /// 返回最终 assistant 消息。
    pub fn final_message(&self) -> Option<&ChatCompletionMessage> {
        self.runner.final_message()
    }

    /// 返回最终 assistant 文本。
    pub fn final_content(&self) -> Option<&str> {
        self.runner.final_content()
    }

    /// 返回最终工具调用。
    pub fn final_function_tool_call(&self) -> Option<&ChatCompletionToolCall> {
        self.runner.final_function_tool_call()
    }

    /// 返回最终工具调用结果。
    pub fn final_function_tool_call_result(&self) -> Option<&str> {
        self.runner.final_function_tool_call_result()
    }

    /// 汇总所有补全响应中的 usage 字段。
    pub fn total_usage(&self) -> Option<Value> {
        self.runner.total_usage()
    }
}

/// 表示 Responses 创建构建器。
#[derive(Debug, Clone, Default)]
pub struct ResponseCreateRequestBuilder {
    client: Option<Client>,
    params: ResponseCreateParams,
    options: RequestOptions,
    extra_body: BTreeMap<String, Value>,
    provider_options: BTreeMap<String, Value>,
}

/// 表示 Responses 流式构建器。
#[derive(Debug, Clone)]
pub struct ResponseStreamRequestBuilder {
    inner: ResponseCreateRequestBuilder,
    response_id: Option<String>,
    starting_after: Option<u64>,
}

/// 表示 Realtime WebSocket 连接构建器。
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
#[derive(Debug, Clone)]
pub struct RealtimeSocketRequestBuilder {
    client: Client,
    model: Option<String>,
    options: RequestOptions,
}

/// 表示 Responses WebSocket 连接构建器。
#[cfg(feature = "responses-ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "responses-ws")))]
#[derive(Debug, Clone)]
pub struct ResponsesSocketRequestBuilder {
    client: Client,
    options: RequestOptions,
}

impl ResponseStreamRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            inner: ResponseCreateRequestBuilder::new(client),
            response_id: None,
            starting_after: None,
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.model(model);
        self
    }

    /// 设置输入文本。
    pub fn input_text(mut self, input: impl Into<String>) -> Self {
        self.inner = self.inner.input_text(input);
        self
    }

    /// 设置输入项数组。
    pub fn input_items(mut self, items: Vec<Value>) -> Self {
        self.inner = self.inner.input_items(items);
        self
    }

    /// 直接设置输入载荷。
    pub fn input(mut self, input: Value) -> Self {
        self.inner = self.inner.input(input);
        self
    }

    /// 设置温度。
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.inner = self.inner.temperature(temperature);
        self
    }

    /// 追加工具定义。
    pub fn tool(mut self, tool: ChatToolDefinition) -> Self {
        self.inner = self.inner.tool(tool);
        self
    }

    /// 添加请求体字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    /// 添加 provider 选项。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    /// 添加额外请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner.options.insert_header(key, value);
        self
    }

    /// 添加额外查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner.options.insert_query(key, value);
        self
    }

    /// 覆盖请求超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner.options.timeout = Some(timeout);
        self
    }

    /// 覆盖最大重试次数。
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.inner.options.max_retries = Some(max_retries);
        self
    }

    /// 设置取消令牌。
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.inner.options.cancellation_token = Some(token);
        self
    }

    /// 按响应 ID 继续一个已有的 Responses SSE 流。
    pub fn response_id(mut self, response_id: impl Into<String>) -> Self {
        self.response_id = Some(response_id.into());
        self
    }

    /// 当继续已有流时，只消费给定序号之后的事件。
    pub fn starting_after(mut self, sequence_number: u64) -> Self {
        self.starting_after = Some(sequence_number);
        self
    }

    /// 发送流式 Responses 请求。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或流初始化失败时返回错误。
    pub async fn send(mut self) -> Result<ResponseStream> {
        let (client, spec) = if let Some(response_id) = self.response_id.take() {
            if self.inner.params.model.is_some()
                || self.inner.params.input.is_some()
                || self.inner.params.temperature.is_some()
                || !self.inner.params.tools.is_empty()
                || !self.inner.extra_body.is_empty()
                || !self.inner.provider_options.is_empty()
            {
                return Err(Error::InvalidConfig(
                    "按 response_id 继续流时，不应再设置创建期参数或请求体扩展字段".into(),
                ));
            }

            let client = self
                .inner
                .client
                .take()
                .ok_or_else(|| Error::InvalidConfig("Responses 构建器缺少客户端".into()))?;
            let mut spec = RequestSpec::new(
                "responses.stream.retrieve",
                Method::GET,
                format!("/responses/{}", encode_path_segment(response_id)),
            );
            spec.options = self.inner.options;
            spec.options.insert_query("stream", "true");
            if let Some(sequence_number) = self.starting_after {
                spec.options
                    .insert_query("starting_after", sequence_number.to_string());
            }
            (client, spec)
        } else {
            if self.starting_after.is_some() {
                return Err(Error::InvalidConfig(
                    "`starting_after` 只能与 `response_id` 一起使用".into(),
                ));
            }
            self.inner.build_spec(true)?
        };
        Ok(ResponseStream::new(client.execute_sse(spec).await?))
    }

    /// 发送流式 Responses 请求，并返回带高层语义事件的运行时流。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或流初始化失败时返回错误。
    pub async fn send_events(self) -> Result<ResponseEventStream> {
        Ok(self.send().await?.events())
    }
}

#[cfg(feature = "realtime")]
impl RealtimeSocketRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            model: None,
            options: RequestOptions::default(),
        }
    }

    /// 设置 Realtime 连接所使用的模型或 deployment。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 添加额外请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_header(key, value);
        self
    }

    /// 添加额外查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_query(key, value);
        self
    }

    /// 建立 Realtime WebSocket 连接。
    ///
    /// # Errors
    ///
    /// 当参数校验失败或握手失败时返回错误。
    pub async fn connect(self) -> Result<RealtimeSocket> {
        RealtimeSocket::connect(&self.client, self.model, self.options).await
    }
}

#[cfg(feature = "responses-ws")]
impl ResponsesSocketRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            options: RequestOptions::default(),
        }
    }

    /// 添加额外请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_header(key, value);
        self
    }

    /// 添加额外查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_query(key, value);
        self
    }

    /// 建立 Responses WebSocket 连接。
    ///
    /// # Errors
    ///
    /// 当握手失败时返回错误。
    pub async fn connect(self) -> Result<ResponsesSocket> {
        ResponsesSocket::connect(&self.client, self.options).await
    }
}

impl ResponseCreateRequestBuilder {
    fn new(client: Client) -> Self {
        Self {
            client: Some(client),
            ..Self::default()
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.params.model = Some(model.into());
        self
    }

    /// 直接设置输入文本。
    pub fn input_text(mut self, input: impl Into<String>) -> Self {
        self.params.input = Some(Value::String(input.into()));
        self
    }

    /// 设置输入项数组。
    pub fn input_items(mut self, items: Vec<Value>) -> Self {
        self.params.input = Some(Value::Array(items));
        self
    }

    /// 直接设置输入载荷。
    pub fn input(mut self, input: Value) -> Self {
        self.params.input = Some(input);
        self
    }

    /// 设置温度。
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.params.temperature = Some(temperature);
        self
    }

    /// 追加工具定义。
    pub fn tool(mut self, tool: ChatToolDefinition) -> Self {
        self.params.tools.push(tool);
        self
    }

    /// 追加请求体字段。
    pub fn extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra_body.insert(key.into(), value);
        self
    }

    /// 追加 provider 选项。
    pub fn provider_option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.provider_options.insert(key.into(), value);
        self
    }

    /// 添加额外请求头。
    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_header(key, value);
        self
    }

    /// 添加额外查询参数。
    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_query(key, value);
        self
    }

    /// 覆盖请求超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    /// 覆盖最大重试次数。
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.options.max_retries = Some(max_retries);
        self
    }

    /// 设置取消令牌。
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.options.cancellation_token = Some(token);
        self
    }

    fn build_spec(mut self, stream: bool) -> Result<(Client, RequestSpec)> {
        let client = self
            .client
            .take()
            .ok_or_else(|| Error::InvalidConfig("Responses 构建器缺少客户端".into()))?;
        if self.params.model.as_deref().unwrap_or_default().is_empty() {
            return Err(Error::MissingRequiredField { field: "model" });
        }
        if self.params.input.is_none() {
            return Err(Error::MissingRequiredField { field: "input" });
        }

        self.params.stream = Some(stream);
        let provider_key = client.provider().kind().as_key();
        let mut body = merge_json_body(
            Some(value_from(&self.params)?),
            &self.extra_body,
            provider_key,
            &self.provider_options,
        );
        if !self.params.tools.is_empty()
            && let Some(object) = body.as_object_mut()
        {
            object.insert(
                "tools".into(),
                Value::Array(
                    self.params
                        .tools
                        .iter()
                        .map(ChatToolDefinition::as_response_tool_value)
                        .collect(),
                ),
            );
        }
        let mut spec = RequestSpec::new(
            if stream {
                "responses.stream"
            } else {
                "responses.create"
            },
            Method::POST,
            "/responses",
        );
        spec.body = Some(body);
        spec.options = self.options;
        Ok((client, spec))
    }

    /// 发送普通 Responses 请求。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或反序列化失败时返回错误。
    pub async fn send(self) -> Result<Response> {
        Ok(self.send_with_meta().await?.data)
    }

    /// 发送普通 Responses 请求并保留元信息。
    ///
    /// # Errors
    ///
    /// 当参数校验失败、请求失败或反序列化失败时返回错误。
    pub async fn send_with_meta(self) -> Result<ApiResponse<Response>> {
        let (client, spec) = self.build_spec(false)?;
        client.execute_json(spec).await
    }
}

/// 表示 Responses 结构化解析构建器。
#[cfg(feature = "structured-output")]
#[derive(Debug, Clone)]
pub struct ResponseParseRequestBuilder<T> {
    inner: ResponseCreateRequestBuilder,
    _marker: PhantomData<T>,
}

#[cfg(feature = "structured-output")]
impl<T> ResponseParseRequestBuilder<T> {
    fn new(client: Client) -> Self {
        Self {
            inner: ResponseCreateRequestBuilder::new(client),
            _marker: PhantomData,
        }
    }

    /// 设置模型。
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.model(model);
        self
    }

    /// 设置输入文本。
    pub fn input_text(mut self, input: impl Into<String>) -> Self {
        self.inner = self.inner.input_text(input);
        self
    }

    /// 设置输入数组。
    pub fn input_items(mut self, items: Vec<Value>) -> Self {
        self.inner = self.inner.input_items(items);
        self
    }
}

#[cfg(feature = "structured-output")]
impl<T> ResponseParseRequestBuilder<T>
where
    T: JsonSchema + serde::de::DeserializeOwned,
{
    /// 发送请求并解析结构化输出。
    ///
    /// # Errors
    ///
    /// 当响应缺少可解析文本或 JSON 解析失败时返回错误。
    pub async fn send(self) -> Result<ParsedResponse<T>> {
        let response = self.inner.send().await?;
        let output_text = response
            .output_text()
            .ok_or_else(|| Error::InvalidConfig("Responses 返回中缺少可解析文本".into()))?;
        let parsed = parse_json_payload(&output_text)?;
        Ok(ParsedResponse { response, parsed })
    }
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

impl CompletionsResource {
    /// 创建 completions 请求构建器。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "completions.create",
            Method::POST,
            "/completions",
        )
    }
}

impl EmbeddingsResource {
    /// 创建 embeddings 请求构建器。
    pub fn create(&self) -> JsonRequestBuilder<EmbeddingResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "embeddings.create",
            Method::POST,
            "/embeddings",
        )
    }
}

impl FilesResource {
    /// 创建文件上传请求。
    pub fn create(&self) -> JsonRequestBuilder<FileObject> {
        JsonRequestBuilder::new(self.client.clone(), "files.create", Method::POST, "/files")
    }

    /// 获取文件对象。
    pub fn retrieve(&self, file_id: impl Into<String>) -> JsonRequestBuilder<FileObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "files.retrieve",
            Method::GET,
            format!("/files/{}", encode_path_segment(file_id.into())),
        )
    }

    /// 列出文件。
    pub fn list(&self) -> ListRequestBuilder<FileObject> {
        ListRequestBuilder::new(self.client.clone(), "files.list", "/files")
    }

    /// 删除文件。
    pub fn delete(&self, file_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "files.delete",
            Method::DELETE,
            format!("/files/{}", encode_path_segment(file_id.into())),
        )
    }

    /// 获取文件内容。
    pub fn content(&self, file_id: impl Into<String>) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "files.content",
            Method::GET,
            format!("/files/{}/content", encode_path_segment(file_id.into())),
        )
    }
}

impl ImagesResource {
    /// 创建图像生成请求。
    pub fn generate(&self) -> ImageGenerateRequestBuilder {
        ImageGenerateRequestBuilder::new(self.client.clone())
    }

    /// 创建图像编辑请求。
    pub fn edit(&self) -> JsonRequestBuilder<ImageGenerationResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "images.edit",
            Method::POST,
            "/images/edits",
        )
    }

    /// 创建图像变体请求。
    pub fn create_variation(&self) -> JsonRequestBuilder<ImageGenerationResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "images.create_variation",
            Method::POST,
            "/images/variations",
        )
    }
}

impl AudioResource {
    /// 返回 speech 子资源。
    pub fn speech(&self) -> AudioSpeechResource {
        AudioSpeechResource::new(self.client.clone())
    }

    /// 返回 transcriptions 子资源。
    pub fn transcriptions(&self) -> AudioTranscriptionsResource {
        AudioTranscriptionsResource::new(self.client.clone())
    }

    /// 返回 translations 子资源。
    pub fn translations(&self) -> AudioTranslationsResource {
        AudioTranslationsResource::new(self.client.clone())
    }
}

impl AudioSpeechResource {
    /// 创建语音合成请求。
    pub fn create(&self) -> AudioSpeechRequestBuilder {
        AudioSpeechRequestBuilder::new(self.client.clone())
    }

    /// 创建 SSE 语音合成请求。
    ///
    /// 该请求会自动在请求体中追加 `stream_format = "sse"`。
    pub fn stream(&self) -> AudioSpeechRequestBuilder {
        AudioSpeechRequestBuilder::stream(self.client.clone())
    }
}

impl AudioTranscriptionsResource {
    /// 创建转写请求。
    pub fn create(&self) -> AudioTranscriptionRequestBuilder {
        AudioTranscriptionRequestBuilder::new(self.client.clone(), false)
    }

    /// 创建流式转写请求。
    ///
    /// 该请求会自动在请求体中追加 `stream = true`。
    pub fn stream(&self) -> AudioTranscriptionRequestBuilder {
        AudioTranscriptionRequestBuilder::new(self.client.clone(), true)
    }
}

impl AudioTranslationsResource {
    /// 创建翻译请求。
    pub fn create(&self) -> AudioTranslationRequestBuilder {
        AudioTranslationRequestBuilder::new(self.client.clone())
    }
}

impl ModerationsResource {
    /// 创建 moderation 请求。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "moderations.create",
            Method::POST,
            "/moderations",
        )
    }
}

impl ModelsResource {
    /// 列出模型。
    pub fn list(&self) -> ListRequestBuilder<Model> {
        ListRequestBuilder::new(self.client.clone(), "models.list", "/models")
    }

    /// 获取单个模型。
    pub fn retrieve(&self, model_id: impl Into<String>) -> JsonRequestBuilder<Model> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "models.retrieve",
            Method::GET,
            format!("/models/{}", encode_path_segment(model_id.into())),
        )
    }

    /// 删除模型。
    pub fn delete(&self, model_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "models.delete",
            Method::DELETE,
            format!("/models/{}", encode_path_segment(model_id.into())),
        )
    }
}

impl FineTuningResource {
    /// 返回 jobs 子资源。
    pub fn jobs(&self) -> FineTuningJobsResource {
        FineTuningJobsResource::new(self.client.clone())
    }

    /// 返回 checkpoints permissions 子资源。
    pub fn checkpoints(&self) -> FineTuningCheckpointPermissionsResource {
        FineTuningCheckpointPermissionsResource::new(self.client.clone())
    }

    /// 返回 alpha 子资源。
    pub fn alpha(&self) -> FineTuningAlphaResource {
        FineTuningAlphaResource::new(self.client.clone())
    }
}

impl FineTuningJobsResource {
    /// 创建 fine-tuning job。
    pub fn create(&self) -> FineTuningJobCreateRequestBuilder {
        FineTuningJobCreateRequestBuilder::new(self.client.clone())
    }

    /// 获取 fine-tuning job。
    pub fn retrieve(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.retrieve",
            Method::GET,
            format!("/fine_tuning/jobs/{}", encode_path_segment(job_id.into())),
        )
    }

    /// 列出 fine-tuning jobs。
    pub fn list(&self) -> ListRequestBuilder<FineTuningJob> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.list",
            "/fine_tuning/jobs",
        )
    }

    /// 取消 fine-tuning job。
    pub fn cancel(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.cancel",
            Method::POST,
            format!(
                "/fine_tuning/jobs/{}/cancel",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 暂停 fine-tuning job。
    pub fn pause(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.pause",
            Method::POST,
            format!(
                "/fine_tuning/jobs/{}/pause",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 恢复 fine-tuning job。
    pub fn resume(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.resume",
            Method::POST,
            format!(
                "/fine_tuning/jobs/{}/resume",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 列出事件。
    pub fn list_events(&self, job_id: impl Into<String>) -> ListRequestBuilder<FineTuningJobEvent> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.list_events",
            format!(
                "/fine_tuning/jobs/{}/events",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 返回 checkpoints 子资源。
    pub fn checkpoints(&self) -> FineTuningJobCheckpointsResource {
        FineTuningJobCheckpointsResource::new(self.client.clone())
    }
}

impl FineTuningJobCheckpointsResource {
    /// 列出某个 job 的 checkpoints。
    pub fn list(&self, job_id: impl Into<String>) -> ListRequestBuilder<FineTuningCheckpoint> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.checkpoints.list",
            format!(
                "/fine_tuning/jobs/{}/checkpoints",
                encode_path_segment(job_id.into())
            ),
        )
    }
}

impl FineTuningCheckpointPermissionsResource {
    /// 创建 checkpoint permission。
    pub fn create(
        &self,
        checkpoint_id: impl Into<String>,
    ) -> JsonRequestBuilder<FineTuningCheckpointPermission> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.create",
            Method::POST,
            format!(
                "/fine_tuning/checkpoints/{}/permissions",
                encode_path_segment(checkpoint_id.into())
            ),
        )
    }

    /// 获取 checkpoint permission。
    pub fn retrieve(
        &self,
        checkpoint_id: impl Into<String>,
        permission_id: impl Into<String>,
    ) -> JsonRequestBuilder<FineTuningCheckpointPermission> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.retrieve",
            Method::GET,
            format!(
                "/fine_tuning/checkpoints/{}/permissions/{}",
                encode_path_segment(checkpoint_id.into()),
                encode_path_segment(permission_id.into())
            ),
        )
    }

    /// 列出 checkpoint permission。
    pub fn list(
        &self,
        checkpoint_id: impl Into<String>,
    ) -> ListRequestBuilder<FineTuningCheckpointPermission> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.list",
            format!(
                "/fine_tuning/checkpoints/{}/permissions",
                encode_path_segment(checkpoint_id.into())
            ),
        )
    }

    /// 删除 checkpoint permission。
    pub fn delete(
        &self,
        checkpoint_id: impl Into<String>,
        permission_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.delete",
            Method::DELETE,
            format!(
                "/fine_tuning/checkpoints/{}/permissions/{}",
                encode_path_segment(checkpoint_id.into()),
                encode_path_segment(permission_id.into())
            ),
        )
    }
}

impl FineTuningAlphaResource {
    /// 返回 graders 子资源。
    pub fn graders(&self) -> FineTuningAlphaGradersResource {
        FineTuningAlphaGradersResource::new(self.client.clone())
    }
}

impl FineTuningAlphaGradersResource {
    /// 执行 grader。
    pub fn run(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.alpha.graders.run",
            Method::POST,
            "/fine_tuning/alpha/graders/run",
        )
    }

    /// 校验 grader。
    pub fn validate(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.alpha.graders.validate",
            Method::POST,
            "/fine_tuning/alpha/graders/validate",
        )
    }
}

impl GradersResource {
    /// 当前资源主要导出类型，暂不提供额外 HTTP 方法。
    pub fn grader_models(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "graders.grader_models",
            Method::GET,
            "/graders/grader_models",
        )
    }
}

impl BatchesResource {
    /// 创建 batch。
    pub fn create(&self) -> BatchCreateRequestBuilder {
        BatchCreateRequestBuilder::new(self.client.clone())
    }

    /// 获取 batch。
    pub fn retrieve(&self, batch_id: impl Into<String>) -> JsonRequestBuilder<Batch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "batches.retrieve",
            Method::GET,
            format!("/batches/{}", encode_path_segment(batch_id.into())),
        )
    }

    /// 列出 batches。
    pub fn list(&self) -> ListRequestBuilder<Batch> {
        ListRequestBuilder::new(self.client.clone(), "batches.list", "/batches")
    }

    /// 取消 batch。
    pub fn cancel(&self, batch_id: impl Into<String>) -> JsonRequestBuilder<Batch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "batches.cancel",
            Method::POST,
            format!("/batches/{}/cancel", encode_path_segment(batch_id.into())),
        )
    }
}

impl UploadsResource {
    /// 创建 upload。
    pub fn create(&self) -> JsonRequestBuilder<UploadObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.create",
            Method::POST,
            "/uploads",
        )
    }

    /// 取消 upload。
    pub fn cancel(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<UploadObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.cancel",
            Method::POST,
            format!("/uploads/{}/cancel", encode_path_segment(upload_id.into())),
        )
    }

    /// 完成 upload。
    pub fn complete(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<UploadObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.complete",
            Method::POST,
            format!(
                "/uploads/{}/complete",
                encode_path_segment(upload_id.into())
            ),
        )
    }

    /// 返回 parts 子资源。
    pub fn parts(&self) -> UploadPartsResource {
        UploadPartsResource::new(self.client.clone())
    }
}

impl UploadPartsResource {
    /// 创建 upload part。
    pub fn create(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.parts.create",
            Method::POST,
            format!("/uploads/{}/parts", encode_path_segment(upload_id.into())),
        )
    }
}

impl ConversationsResource {
    /// 创建 conversation。
    pub fn create(&self) -> JsonRequestBuilder<Conversation> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.create",
            Method::POST,
            "/conversations",
        )
    }

    /// 获取 conversation。
    pub fn retrieve(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<Conversation> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.retrieve",
            Method::GET,
            format!(
                "/conversations/{}",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 更新 conversation。
    pub fn update(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<Conversation> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.update",
            Method::POST,
            format!(
                "/conversations/{}",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 删除 conversation。
    pub fn delete(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.delete",
            Method::DELETE,
            format!(
                "/conversations/{}",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 返回 items 子资源。
    pub fn items(&self) -> ConversationItemsResource {
        ConversationItemsResource::new(self.client.clone())
    }
}

impl ConversationItemsResource {
    /// 创建 conversation item。
    pub fn create(
        &self,
        conversation_id: impl Into<String>,
    ) -> JsonRequestBuilder<ConversationItem> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.items.create",
            Method::POST,
            format!(
                "/conversations/{}/items",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 获取 conversation item。
    pub fn retrieve(
        &self,
        conversation_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<ConversationItem> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.items.retrieve",
            Method::GET,
            format!(
                "/conversations/{}/items/{}",
                encode_path_segment(conversation_id.into()),
                encode_path_segment(item_id.into())
            ),
        )
    }

    /// 列出 conversation items。
    pub fn list(&self, conversation_id: impl Into<String>) -> ListRequestBuilder<ConversationItem> {
        ListRequestBuilder::new(
            self.client.clone(),
            "conversations.items.list",
            format!(
                "/conversations/{}/items",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 删除 conversation item。
    pub fn delete(
        &self,
        conversation_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.items.delete",
            Method::DELETE,
            format!(
                "/conversations/{}/items/{}",
                encode_path_segment(conversation_id.into()),
                encode_path_segment(item_id.into())
            ),
        )
    }
}

impl EvalsResource {
    /// 创建 eval。
    pub fn create(&self) -> JsonRequestBuilder<Eval> {
        JsonRequestBuilder::new(self.client.clone(), "evals.create", Method::POST, "/evals")
    }

    /// 获取 eval。
    pub fn retrieve(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<Eval> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.retrieve",
            Method::GET,
            format!("/evals/{}", encode_path_segment(eval_id.into())),
        )
    }

    /// 更新 eval。
    pub fn update(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<Eval> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.update",
            Method::POST,
            format!("/evals/{}", encode_path_segment(eval_id.into())),
        )
    }

    /// 列出 evals。
    pub fn list(&self) -> ListRequestBuilder<Eval> {
        ListRequestBuilder::new(self.client.clone(), "evals.list", "/evals")
    }

    /// 删除 eval。
    pub fn delete(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.delete",
            Method::DELETE,
            format!("/evals/{}", encode_path_segment(eval_id.into())),
        )
    }

    /// 返回 runs 子资源。
    pub fn runs(&self) -> EvalRunsResource {
        EvalRunsResource::new(self.client.clone())
    }
}

impl EvalRunsResource {
    /// 创建 eval run。
    pub fn create(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<EvalRun> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.create",
            Method::POST,
            format!("/evals/{}/runs", encode_path_segment(eval_id.into())),
        )
    }

    /// 获取 eval run。
    pub fn retrieve(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalRun> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.retrieve",
            Method::GET,
            format!(
                "/evals/{}/runs/{}",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 列出 eval runs。
    pub fn list(&self, eval_id: impl Into<String>) -> ListRequestBuilder<EvalRun> {
        ListRequestBuilder::new(
            self.client.clone(),
            "evals.runs.list",
            format!("/evals/{}/runs", encode_path_segment(eval_id.into())),
        )
    }

    /// 删除 eval run。
    pub fn delete(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.delete",
            Method::DELETE,
            format!(
                "/evals/{}/runs/{}",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 取消 eval run。
    pub fn cancel(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalRun> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.cancel",
            Method::POST,
            format!(
                "/evals/{}/runs/{}/cancel",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 返回 output_items 子资源。
    pub fn output_items(&self) -> EvalRunOutputItemsResource {
        EvalRunOutputItemsResource::new(self.client.clone())
    }
}

impl EvalRunOutputItemsResource {
    /// 获取 output item。
    pub fn retrieve(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalOutputItem> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.output_items.retrieve",
            Method::GET,
            format!(
                "/evals/{}/runs/{}/output_items/{}",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into()),
                encode_path_segment(item_id.into())
            ),
        )
    }

    /// 列出 output items。
    pub fn list(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> ListRequestBuilder<EvalOutputItem> {
        ListRequestBuilder::new(
            self.client.clone(),
            "evals.runs.output_items.list",
            format!(
                "/evals/{}/runs/{}/output_items",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }
}

impl ContainersResource {
    /// 创建 container。
    pub fn create(&self) -> JsonRequestBuilder<Container> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.create",
            Method::POST,
            "/containers",
        )
    }

    /// 获取 container。
    pub fn retrieve(&self, container_id: impl Into<String>) -> JsonRequestBuilder<Container> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.retrieve",
            Method::GET,
            format!("/containers/{}", encode_path_segment(container_id.into())),
        )
    }

    /// 列出 containers。
    pub fn list(&self) -> ListRequestBuilder<Container> {
        ListRequestBuilder::new(self.client.clone(), "containers.list", "/containers")
    }

    /// 删除 container。
    pub fn delete(&self, container_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.delete",
            Method::DELETE,
            format!("/containers/{}", encode_path_segment(container_id.into())),
        )
    }

    /// 返回 files 子资源。
    pub fn files(&self) -> ContainerFilesResource {
        ContainerFilesResource::new(self.client.clone())
    }
}

impl ContainerFilesResource {
    /// 创建 container file。
    pub fn create(&self, container_id: impl Into<String>) -> JsonRequestBuilder<ContainerFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.files.create",
            Method::POST,
            format!(
                "/containers/{}/files",
                encode_path_segment(container_id.into())
            ),
        )
    }

    /// 获取 container file。
    pub fn retrieve(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<ContainerFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.files.retrieve",
            Method::GET,
            format!(
                "/containers/{}/files/{}",
                encode_path_segment(container_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 列出 container files。
    pub fn list(&self, container_id: impl Into<String>) -> ListRequestBuilder<ContainerFile> {
        ListRequestBuilder::new(
            self.client.clone(),
            "containers.files.list",
            format!(
                "/containers/{}/files",
                encode_path_segment(container_id.into())
            ),
        )
    }

    /// 删除 container file。
    pub fn delete(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.files.delete",
            Method::DELETE,
            format!(
                "/containers/{}/files/{}",
                encode_path_segment(container_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 返回 content 子资源。
    pub fn content(&self) -> ContainerFilesContentResource {
        ContainerFilesContentResource::new(self.client.clone())
    }
}

impl ContainerFilesContentResource {
    /// 获取 container file 内容。
    pub fn retrieve(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "containers.files.content.retrieve",
            Method::GET,
            format!(
                "/containers/{}/files/{}/content",
                encode_path_segment(container_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }
}

impl SkillsResource {
    /// 创建 skill。
    pub fn create(&self) -> JsonRequestBuilder<Skill> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.create",
            Method::POST,
            "/skills",
        )
    }

    /// 获取 skill。
    pub fn retrieve(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<Skill> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.retrieve",
            Method::GET,
            format!("/skills/{}", encode_path_segment(skill_id.into())),
        )
    }

    /// 更新 skill。
    pub fn update(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<Skill> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.update",
            Method::POST,
            format!("/skills/{}", encode_path_segment(skill_id.into())),
        )
    }

    /// 列出 skills。
    pub fn list(&self) -> ListRequestBuilder<Skill> {
        ListRequestBuilder::new(self.client.clone(), "skills.list", "/skills")
    }

    /// 删除 skill。
    pub fn delete(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.delete",
            Method::DELETE,
            format!("/skills/{}", encode_path_segment(skill_id.into())),
        )
    }

    /// 返回 content 子资源。
    pub fn content(&self) -> SkillsContentResource {
        SkillsContentResource::new(self.client.clone())
    }

    /// 返回 versions 子资源。
    pub fn versions(&self) -> SkillVersionsResource {
        SkillVersionsResource::new(self.client.clone())
    }
}

impl SkillsContentResource {
    /// 获取 skill 内容。
    pub fn retrieve(&self, skill_id: impl Into<String>) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "skills.content.retrieve",
            Method::GET,
            format!("/skills/{}/content", encode_path_segment(skill_id.into())),
        )
    }
}

impl SkillVersionsResource {
    /// 创建 skill version。
    pub fn create(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<SkillVersion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.versions.create",
            Method::POST,
            format!("/skills/{}/versions", encode_path_segment(skill_id.into())),
        )
    }

    /// 获取 skill version。
    pub fn retrieve(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> JsonRequestBuilder<SkillVersion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.versions.retrieve",
            Method::GET,
            format!(
                "/skills/{}/versions/{}",
                encode_path_segment(skill_id.into()),
                encode_path_segment(version_id.into())
            ),
        )
    }

    /// 列出 skill versions。
    pub fn list(&self, skill_id: impl Into<String>) -> ListRequestBuilder<SkillVersion> {
        ListRequestBuilder::new(
            self.client.clone(),
            "skills.versions.list",
            format!("/skills/{}/versions", encode_path_segment(skill_id.into())),
        )
    }

    /// 删除 skill version。
    pub fn delete(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.versions.delete",
            Method::DELETE,
            format!(
                "/skills/{}/versions/{}",
                encode_path_segment(skill_id.into()),
                encode_path_segment(version_id.into())
            ),
        )
    }

    /// 返回 content 子资源。
    pub fn content(&self) -> SkillVersionsContentResource {
        SkillVersionsContentResource::new(self.client.clone())
    }
}

impl SkillVersionsContentResource {
    /// 获取 skill version 内容。
    pub fn retrieve(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "skills.versions.content.retrieve",
            Method::GET,
            format!(
                "/skills/{}/versions/{}/content",
                encode_path_segment(skill_id.into()),
                encode_path_segment(version_id.into())
            ),
        )
    }
}

impl VideosResource {
    /// 创建视频。
    pub fn create(&self) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.create",
            Method::POST,
            "/videos",
        )
    }

    /// 获取视频。
    pub fn retrieve(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.retrieve",
            Method::GET,
            format!("/videos/{}", encode_path_segment(video_id.into())),
        )
    }

    /// 列出视频。
    pub fn list(&self) -> ListRequestBuilder<Video> {
        ListRequestBuilder::new(self.client.clone(), "videos.list", "/videos")
    }

    /// 删除视频。
    pub fn delete(&self, video_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.delete",
            Method::DELETE,
            format!("/videos/{}", encode_path_segment(video_id.into())),
        )
    }

    /// 编辑视频。
    pub fn edit(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.edit",
            Method::POST,
            format!("/videos/{}/edit", encode_path_segment(video_id.into())),
        )
    }

    /// 扩展视频。
    pub fn extend(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.extend",
            Method::POST,
            format!("/videos/{}/extend", encode_path_segment(video_id.into())),
        )
    }

    /// 创建角色。
    pub fn create_character(&self) -> JsonRequestBuilder<VideoCharacter> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.create_character",
            Method::POST,
            "/videos/characters",
        )
    }

    /// 获取角色。
    pub fn get_character(
        &self,
        character_id: impl Into<String>,
    ) -> JsonRequestBuilder<VideoCharacter> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.get_character",
            Method::GET,
            format!(
                "/videos/characters/{}",
                encode_path_segment(character_id.into())
            ),
        )
    }

    /// 下载视频内容。
    pub fn download_content(&self, video_id: impl Into<String>) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "videos.download_content",
            Method::GET,
            format!("/videos/{}/content", encode_path_segment(video_id.into())),
        )
    }

    /// 混剪视频。
    pub fn remix(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.remix",
            Method::POST,
            format!("/videos/{}/remix", encode_path_segment(video_id.into())),
        )
    }
}

impl WebhooksResource {
    fn verifier(&self) -> WebhookVerifier {
        WebhookVerifier::new(self.client.inner.options.webhook_secret.clone())
    }

    /// 校验 Webhook 签名。
    ///
    /// # Errors
    ///
    /// 当签名不合法时返回错误。
    pub fn verify_signature<H>(
        &self,
        payload: &str,
        headers: &H,
        secret: Option<&str>,
        tolerance: Duration,
    ) -> Result<()>
    where
        H: HeaderLookup,
    {
        self.verifier()
            .verify_signature(payload, headers, secret, tolerance)
    }

    /// 校验签名并解包事件。
    ///
    /// # Errors
    ///
    /// 当签名校验失败或 JSON 解析失败时返回错误。
    pub fn unwrap<H, T>(
        &self,
        payload: &str,
        headers: &H,
        secret: Option<&str>,
        tolerance: Duration,
    ) -> Result<T>
    where
        H: HeaderLookup,
        T: serde::de::DeserializeOwned,
    {
        self.verifier().unwrap(payload, headers, secret, tolerance)
    }
}

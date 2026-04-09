//! Chat namespace implementations, builders, and tool-runner helpers.

use std::collections::BTreeMap;
#[cfg(feature = "structured-output")]
use std::marker::PhantomData;
use std::time::Duration;

use bytes::Bytes;
use http::Method;
#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
#[cfg(feature = "tool-runner")]
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::Client;
use crate::config::RequestOptions;
use crate::error::{Error, Result};
use crate::generated::endpoints;
#[cfg(feature = "structured-output")]
use crate::helpers::{ParsedChatCompletion, parse_json_payload};
#[cfg(feature = "tool-runner")]
use crate::helpers::{ToolDefinition, ToolRegistry};
use crate::json_payload::JsonPayload;
use crate::response_meta::ApiResponse;
#[cfg(feature = "tool-runner")]
use crate::stream::ChatCompletionRuntimeEvent;
use crate::stream::{
    AssistantEventStream, AssistantStream, ChatCompletionEventStream, ChatCompletionStream,
};
use crate::transport::{RequestSpec, merge_json_body};
#[cfg(feature = "tool-runner")]
use futures_util::StreamExt;

#[cfg(feature = "tool-runner")]
use super::ChatCompletionToolCall;
#[cfg(feature = "tool-runner")]
use super::CompletionUsage;
use super::{
    ChatCompletion, ChatCompletionCreateParams, ChatCompletionMessage,
    ChatCompletionMessagesResource, ChatCompletionStoreContentPart, ChatCompletionsResource,
    ChatResource, ChatToolChoice, ChatToolDefinition, DeleteResponse, JsonRequestBuilder,
    ListRequestBuilder, encode_path_segment, value_from,
};

/// 表示已存储 chat completion 下的消息对象。
#[derive(Debug, Clone, Serialize, serde::Deserialize, Default)]
pub struct ChatCompletionStoreMessage {
    /// 消息 ID。
    pub id: String,
    /// 角色。
    #[serde(default)]
    pub role: String,
    /// 文本内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// content parts。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_parts: Vec<ChatCompletionStoreContentPart>,
    /// 工具调用。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<super::ChatCompletionToolCall>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatResource {
    /// 返回聊天补全资源。
    pub fn completions(&self) -> ChatCompletionsResource {
        ChatCompletionsResource::new(self.client.clone())
    }
}

impl ChatCompletionsResource {
    /// 创建聊天补全请求构建器。
    pub fn create(&self) -> ChatCompletionCreateRequestBuilder {
        ChatCompletionCreateRequestBuilder::new(self.client.clone())
    }

    /// 创建聊天补全流式请求构建器。
    pub fn stream(&self) -> ChatCompletionStreamRequestBuilder {
        ChatCompletionStreamRequestBuilder::new(self.client.clone())
    }

    /// 创建结构化解析请求构建器。
    #[cfg(feature = "structured-output")]
    #[cfg_attr(docsrs, doc(cfg(feature = "structured-output")))]
    pub fn parse<T>(&self) -> ChatCompletionParseRequestBuilder<T> {
        ChatCompletionParseRequestBuilder::new(self.client.clone())
    }

    /// 创建工具运行构建器。
    #[cfg(feature = "tool-runner")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
    pub fn run_tools(&self) -> ChatCompletionRunToolsRequestBuilder {
        ChatCompletionRunToolsRequestBuilder::new(self.client.clone())
    }

    /// 根据 ID 获取聊天补全对象。
    pub fn retrieve(&self, id: impl Into<String>) -> JsonRequestBuilder<ChatCompletion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "chat.completions.retrieve",
            Method::GET,
            format!("/chat/completions/{}", encode_path_segment(id.into())),
        )
    }

    /// 更新聊天补全对象。
    pub fn update(&self, id: impl Into<String>) -> JsonRequestBuilder<ChatCompletion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "chat.completions.update",
            Method::POST,
            format!("/chat/completions/{}", encode_path_segment(id.into())),
        )
    }

    /// 列出聊天补全对象。
    pub fn list(&self) -> ListRequestBuilder<ChatCompletion> {
        ListRequestBuilder::new(
            self.client.clone(),
            "chat.completions.list",
            "/chat/completions",
        )
    }

    /// 删除聊天补全对象。
    pub fn delete(&self, id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "chat.completions.delete",
            Method::DELETE,
            format!("/chat/completions/{}", encode_path_segment(id.into())),
        )
    }

    /// 返回聊天补全消息子资源。
    pub fn messages(&self) -> ChatCompletionMessagesResource {
        ChatCompletionMessagesResource::new(self.client.clone())
    }
}

impl ChatCompletionMessagesResource {
    /// 列出某个聊天补全下的消息。
    pub fn list(
        &self,
        completion_id: impl Into<String>,
    ) -> ListRequestBuilder<ChatCompletionStoreMessage> {
        let endpoint = endpoints::chat::CHAT_COMPLETIONS_MESSAGES_LIST;
        ListRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("completion_id", &encode_path_segment(completion_id.into()))]),
        )
    }
}

/// 表示聊天补全创建构建器。
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionCreateRequestBuilder {
    client: Option<Client>,
    pub(crate) params: ChatCompletionCreateParams,
    options: RequestOptions,
    extra_body: BTreeMap<String, Value>,
    provider_options: BTreeMap<String, Value>,
}

impl ChatCompletionCreateRequestBuilder {
    pub(crate) fn new(client: Client) -> Self {
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
    pub fn tool_choice(mut self, tool_choice: impl Into<ChatToolChoice>) -> Self {
        self.params.tool_choice = Some(tool_choice.into());
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.extra_body.insert(key.into(), value.into().into_raw());
        self
    }

    /// 在 provider 对应的 `provider_options` 节点下追加字段。
    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.provider_options
            .insert(key.into(), value.into().into_raw());
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

    pub(crate) fn build_spec(mut self, stream: bool) -> Result<(Client, RequestSpec)> {
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
    pub(crate) fn new(client: Client) -> Self {
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    /// 添加 provider 选项。
    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
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
    pub(crate) fn new(
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
    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    /// 在 provider 对应的 `provider_options` 下追加字段。
    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
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
    pub(crate) fn new(client: Client) -> Self {
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
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
    pub(crate) fn new(client: Client) -> Self {
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
    pub fn total_usage(&self) -> Option<CompletionUsage> {
        let mut completion_tokens = 0u64;
        let mut prompt_tokens = 0u64;
        let mut total_tokens = 0u64;
        let mut found = false;
        for completion in &self.chat_completions {
            let Some(usage) = completion.usage.as_ref() else {
                continue;
            };
            completion_tokens += usage.completion_tokens;
            prompt_tokens += usage.prompt_tokens;
            total_tokens += usage.total_tokens;
            found = true;
        }

        found.then(|| CompletionUsage {
            completion_tokens,
            prompt_tokens,
            total_tokens,
            ..CompletionUsage::default()
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
    pub fn total_usage(&self) -> Option<CompletionUsage> {
        self.runner.total_usage()
    }
}

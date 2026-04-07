//! Responses and Realtime namespace implementations and builders.

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::time::Duration;

use http::Method;
#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::Client;
use crate::config::RequestOptions;
use crate::error::{Error, Result};
#[cfg(feature = "structured-output")]
use crate::helpers::{ParsedResponse, parse_json_payload};
use crate::response_meta::ApiResponse;
use crate::stream::{ResponseEventStream, ResponseStream};
use crate::transport::{RequestSpec, merge_json_body};
#[cfg(feature = "realtime")]
use crate::websocket::RealtimeSocket;
#[cfg(feature = "responses-ws")]
use crate::websocket::ResponsesSocket;

use super::{
    ChatToolDefinition, DeleteResponse, InputTokenCount, JsonRequestBuilder, ListRequestBuilder,
    RealtimeCallsResource, RealtimeClientSecretsResource, RealtimeResource, Response,
    ResponseCreateParams, ResponseInputItemsResource, ResponseInputTokensResource,
    ResponsesResource, encode_path_segment, value_from,
};

impl ResponsesResource {
    /// 创建 responses 请求构建器。
    pub fn create(&self) -> ResponseCreateRequestBuilder {
        ResponseCreateRequestBuilder::new(self.client.clone())
    }

    /// 创建 responses 结构化解析构建器。
    #[cfg(feature = "structured-output")]
    #[cfg_attr(docsrs, doc(cfg(feature = "structured-output")))]
    pub fn parse<T>(&self) -> ResponseParseRequestBuilder<T> {
        ResponseParseRequestBuilder::new(self.client.clone())
    }

    /// 创建 responses 流式构建器。
    pub fn stream(&self) -> ResponseStreamRequestBuilder {
        ResponseStreamRequestBuilder::new(self.client.clone())
    }

    /// 按响应 ID 继续一个已有的 Responses SSE 流。
    pub fn stream_response(&self, response_id: impl Into<String>) -> ResponseStreamRequestBuilder {
        ResponseStreamRequestBuilder::new(self.client.clone()).response_id(response_id)
    }

    /// 创建 Responses WebSocket 连接构建器。
    #[cfg(feature = "responses-ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "responses-ws")))]
    pub fn ws(&self) -> ResponsesSocketRequestBuilder {
        ResponsesSocketRequestBuilder::new(self.client.clone())
    }

    /// 获取 response。
    pub fn retrieve(&self, response_id: impl Into<String>) -> JsonRequestBuilder<Response> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "responses.retrieve",
            Method::GET,
            format!("/responses/{}", encode_path_segment(response_id.into())),
        )
    }

    /// 删除 response。
    pub fn delete(&self, response_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "responses.delete",
            Method::DELETE,
            format!("/responses/{}", encode_path_segment(response_id.into())),
        )
    }

    /// 取消后台 response。
    pub fn cancel(&self, response_id: impl Into<String>) -> JsonRequestBuilder<Response> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "responses.cancel",
            Method::POST,
            format!(
                "/responses/{}/cancel",
                encode_path_segment(response_id.into())
            ),
        )
    }

    /// 压缩 response。
    pub fn compact(&self, response_id: impl Into<String>) -> JsonRequestBuilder<Response> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "responses.compact",
            Method::POST,
            format!(
                "/responses/{}/compact",
                encode_path_segment(response_id.into())
            ),
        )
    }

    /// 返回 input_items 子资源。
    pub fn input_items(&self) -> ResponseInputItemsResource {
        ResponseInputItemsResource::new(self.client.clone())
    }

    /// 返回 input_tokens 子资源。
    pub fn input_tokens(&self) -> ResponseInputTokensResource {
        ResponseInputTokensResource::new(self.client.clone())
    }
}

impl ResponseInputItemsResource {
    /// 列出 response 输入项。
    pub fn list(&self, response_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "responses.input_items.list",
            format!(
                "/responses/{}/input_items",
                encode_path_segment(response_id.into())
            ),
        )
    }
}

impl ResponseInputTokensResource {
    /// 统计输入 token。
    pub fn count(&self) -> JsonRequestBuilder<InputTokenCount> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "responses.input_tokens.count",
            Method::POST,
            "/responses/input_tokens",
        )
    }
}

impl RealtimeResource {
    /// 创建 Realtime WebSocket 连接构建器。
    #[cfg(feature = "realtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
    pub fn ws(&self) -> RealtimeSocketRequestBuilder {
        RealtimeSocketRequestBuilder::new(self.client.clone())
    }

    /// 返回 client_secrets 子资源。
    pub fn client_secrets(&self) -> RealtimeClientSecretsResource {
        RealtimeClientSecretsResource::new(self.client.clone())
    }

    /// 返回 calls 子资源。
    pub fn calls(&self) -> RealtimeCallsResource {
        RealtimeCallsResource::new(self.client.clone())
    }
}

impl RealtimeClientSecretsResource {
    /// 创建 client secret。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "realtime.client_secrets.create",
            Method::POST,
            "/realtime/client_secrets",
        )
    }
}

impl RealtimeCallsResource {
    /// 接听通话。
    pub fn accept(&self, call_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        realtime_call_action(
            self.client.clone(),
            "realtime.calls.accept",
            call_id,
            "accept",
        )
    }

    /// 挂断通话。
    pub fn hangup(&self, call_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        realtime_call_action(
            self.client.clone(),
            "realtime.calls.hangup",
            call_id,
            "hangup",
        )
    }

    /// 转接通话。
    pub fn refer(&self, call_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        realtime_call_action(
            self.client.clone(),
            "realtime.calls.refer",
            call_id,
            "refer",
        )
    }

    /// 拒绝通话。
    pub fn reject(&self, call_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        realtime_call_action(
            self.client.clone(),
            "realtime.calls.reject",
            call_id,
            "reject",
        )
    }
}

fn realtime_call_action(
    client: Client,
    endpoint_id: &'static str,
    call_id: impl Into<String>,
    action: &str,
) -> JsonRequestBuilder<Value> {
    JsonRequestBuilder::new(
        client,
        endpoint_id,
        Method::POST,
        format!(
            "/realtime/calls/{}/{}",
            encode_path_segment(call_id.into()),
            action
        ),
    )
}

/// 表示 Responses 创建构建器。
#[derive(Debug, Clone, Default)]
pub struct ResponseCreateRequestBuilder {
    client: Option<Client>,
    pub(crate) params: ResponseCreateParams,
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
    pub(crate) fn new(client: Client) -> Self {
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
    pub(crate) fn new(client: Client) -> Self {
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
    pub(crate) fn new(client: Client) -> Self {
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

    pub(crate) fn build_spec(mut self, stream: bool) -> Result<(Client, RequestSpec)> {
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
    pub(crate) fn new(client: Client) -> Self {
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

//! Responses 与 Realtime 命名空间实现。

use http::Method;
use serde_json::Value;

use crate::Client;

#[cfg(feature = "realtime")]
use super::RealtimeSocketRequestBuilder;
#[cfg(feature = "structured-output")]
use super::ResponseParseRequestBuilder;
#[cfg(feature = "responses-ws")]
use super::ResponsesSocketRequestBuilder;
use super::{DeleteResponse, InputTokenCount, JsonRequestBuilder, ListRequestBuilder, Response};
use super::{
    RealtimeCallsResource, RealtimeClientSecretsResource, RealtimeResource,
    ResponseCreateRequestBuilder, ResponseInputItemsResource, ResponseInputTokensResource,
    ResponseStreamRequestBuilder, ResponsesResource, encode_path_segment,
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

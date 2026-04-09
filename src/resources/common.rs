//! Shared request-builder primitives used across resource namespaces.

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::time::Duration;

use bytes::Bytes;
use http::Method;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::Client;
use crate::config::RequestOptions;
use crate::error::{Error, Result};
use crate::files::{MultipartField, UploadSource};
use crate::json_payload::JsonPayload;
use crate::pagination::{CursorPage, ListEnvelope};
use crate::response_meta::ApiResponse;
use crate::stream::{RawSseStream, SseStream};
use crate::transport::{RequestSpec, merge_json_body};

/// URL path encoding set used for dynamic path segments.
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b'=');

pub(crate) fn value_from<T>(value: &T) -> Result<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|error| {
        crate::error::Error::Serialization(crate::SerializationError::new(error.to_string()))
    })
}

/// 对单个路径参数做安全编码，避免动态 ID 改写 URL 结构。
pub(crate) fn encode_path_segment(segment: impl AsRef<str>) -> String {
    utf8_percent_encode(segment.as_ref(), PATH_SEGMENT_ENCODE_SET).to_string()
}

pub(crate) fn metadata_is_empty(metadata: &BTreeMap<String, String>) -> bool {
    metadata.is_empty()
}

/// Shared state for typed JSON request builders defined in longtail namespaces.
#[derive(Debug, Clone)]
pub(crate) struct TypedJsonRequestState<P> {
    pub(crate) client: Option<Client>,
    pub(crate) params: P,
    pub(crate) body_override: Option<Value>,
    pub(crate) options: RequestOptions,
    pub(crate) extra_body: BTreeMap<String, Value>,
    pub(crate) provider_options: BTreeMap<String, Value>,
}

impl<P> TypedJsonRequestState<P> {
    pub(crate) fn new(client: Client, params: P) -> Self {
        Self {
            client: Some(client),
            params,
            body_override: None,
            options: RequestOptions::default(),
            extra_body: BTreeMap::new(),
            provider_options: BTreeMap::new(),
        }
    }

    pub(crate) fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.body_override = Some(body.into().into_raw());
        self
    }

    pub(crate) fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_header(key, value);
        self
    }

    pub(crate) fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert_query(key, value);
        self
    }

    pub(crate) fn extra_body(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.extra_body.insert(key.into(), value.into().into_raw());
        self
    }

    pub(crate) fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.provider_options
            .insert(key.into(), value.into().into_raw());
        self
    }

    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    pub(crate) fn max_retries(mut self, max_retries: u32) -> Self {
        self.options.max_retries = Some(max_retries);
        self
    }

    pub(crate) fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.options.cancellation_token = Some(token);
        self
    }
}

impl<P> TypedJsonRequestState<P>
where
    P: Serialize,
{
    pub(crate) fn build_spec(
        mut self,
        endpoint_id: &'static str,
        path: &'static str,
    ) -> Result<(Client, RequestSpec)> {
        let client = self
            .client
            .take()
            .ok_or_else(|| Error::InvalidConfig("请求构建器缺少客户端".into()))?;
        let provider_key = client.provider().kind().as_key();
        let body = merge_json_body(
            Some(
                self.body_override
                    .take()
                    .unwrap_or(value_from(&self.params)?),
            ),
            &self.extra_body,
            provider_key,
            &self.provider_options,
        );
        let mut spec = RequestSpec::new(endpoint_id, Method::POST, path);
        spec.body = Some(body);
        spec.options = self.options;
        Ok((client, spec))
    }
}

/// 表示通用 JSON 请求构建器。
#[derive(Debug, Clone)]
pub struct JsonRequestBuilder<T> {
    pub(crate) client: Client,
    pub(crate) spec: RequestSpec,
    pub(crate) extra_body: BTreeMap<String, Value>,
    pub(crate) provider_options: BTreeMap<String, Value>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> JsonRequestBuilder<T> {
    pub(crate) fn new(
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
    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.spec.body = Some(body.into().into_raw());
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.extra_body.insert(key.into(), value.into().into_raw());
        self
    }

    /// 在 provider 对应的 `provider_options` 下追加字段。
    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.provider_options
            .insert(key.into(), value.into().into_raw());
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

    pub(crate) fn into_spec(mut self) -> RequestSpec {
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
    pub(crate) inner: JsonRequestBuilder<Bytes>,
}

/// 表示不关心响应体的请求构建器。
#[derive(Debug, Clone)]
pub struct NoContentRequestBuilder {
    pub(crate) inner: JsonRequestBuilder<Bytes>,
}

impl BytesRequestBuilder {
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

    /// 设置 JSON 请求体。
    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
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

impl NoContentRequestBuilder {
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

    /// 设置 JSON 请求体。
    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
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

    /// 删除一个默认请求头。
    pub fn remove_header(mut self, key: impl Into<String>) -> Self {
        self.inner = self.inner.remove_header(key);
        self
    }

    /// 追加查询参数。
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

    /// 发送请求并忽略响应体。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send(self) -> Result<()> {
        self.send_with_meta().await.map(|_| ())
    }

    /// 发送请求并保留响应元信息。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn send_with_meta(self) -> Result<ApiResponse<()>> {
        let client = self.inner.client.clone();
        let response = client.execute_bytes(self.inner.into_spec()).await?;
        let (_, meta) = response.into_parts();
        Ok(ApiResponse::new((), meta))
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
}

/// 表示列表请求构建器。
#[derive(Debug, Clone)]
pub struct ListRequestBuilder<T> {
    pub(crate) inner: JsonRequestBuilder<ListEnvelope<T>>,
}

impl<T> ListRequestBuilder<T> {
    pub(crate) fn new(client: Client, endpoint_id: &'static str, path: impl Into<String>) -> Self {
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
    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
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

#[cfg(test)]
mod tests {
    use percent_encoding::percent_decode_str;
    use proptest::prelude::*;

    use super::encode_path_segment;

    proptest! {
        #[test]
        fn encoded_path_segment_roundtrips_through_percent_decode(segment in any::<String>()) {
            let encoded = encode_path_segment(&segment);
            let decoded = percent_decode_str(&encoded).decode_utf8().unwrap();
            prop_assert_eq!(decoded, segment);
        }
    }
}

//! HTTP 传输与重试逻辑。

use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use http::Method;
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tokio::time::timeout;
use tracing::instrument;

use crate::auth::ApiKeySource;
use crate::client::ClientInner;
use crate::config::{LogLevel, RequestOptions};
use crate::error::{ApiError, ConnectionError, Error, Result};
use crate::files::{MultipartField, UploadSource};
use crate::json_payload::JsonPayload;
use crate::providers::{AuthScheme, RequestContext};
use crate::response_meta::{ApiResponse, ResponseMeta, into_http_response};
use crate::stream::{RawSseStream, SseStream};

/// 表示 Multipart 负载。
#[derive(Debug, Clone, Default)]
pub struct MultipartPayload {
    /// 文本字段。
    pub fields: Vec<MultipartField>,
    /// 文件字段，键为字段名。
    pub files: Vec<(String, UploadSource)>,
}

/// 表示一次标准化后的请求规格。
#[derive(Debug, Clone)]
pub struct RequestSpec {
    /// 逻辑端点 ID。
    pub endpoint_id: &'static str,
    /// HTTP 方法。
    pub method: Method,
    /// 请求路径。
    pub path: String,
    /// JSON 请求体。
    pub body: Option<Value>,
    /// 请求选项。
    pub options: RequestOptions,
    /// Multipart 负载。
    pub multipart: Option<MultipartPayload>,
}

impl RequestSpec {
    /// 创建新的请求规格。
    pub fn new(endpoint_id: &'static str, method: Method, path: impl Into<String>) -> Self {
        Self {
            endpoint_id,
            method,
            path: path.into(),
            body: None,
            options: RequestOptions::default(),
            multipart: None,
        }
    }
}

/// 发送 JSON 请求并解析返回值。
#[instrument(skip(inner, spec), fields(endpoint_id = spec.endpoint_id, provider = ?inner.provider.kind()))]
pub(crate) async fn execute_json<T>(
    inner: &ClientInner,
    spec: RequestSpec,
) -> Result<ApiResponse<T>>
where
    T: DeserializeOwned,
{
    let response = execute(inner, spec).await?;
    let (bytes, meta) = response;
    let parsed = serde_json::from_slice::<T>(&bytes).map_err(|error| {
        Error::Serialization(crate::SerializationError::new(format!(
            "JSON 反序列化失败: {error}"
        )))
    })?;
    Ok(ApiResponse::new(parsed, meta))
}

/// 发送请求并返回原始字节。
#[instrument(skip(inner, spec), fields(endpoint_id = spec.endpoint_id, provider = ?inner.provider.kind()))]
pub(crate) async fn execute_bytes(
    inner: &ClientInner,
    spec: RequestSpec,
) -> Result<ApiResponse<Bytes>> {
    let (bytes, meta) = execute(inner, spec).await?;
    Ok(ApiResponse::new(bytes, meta))
}

/// 发送 SSE 请求并返回类型化流。
#[instrument(skip(inner, spec), fields(endpoint_id = spec.endpoint_id, provider = ?inner.provider.kind()))]
pub(crate) async fn execute_sse<T>(inner: &ClientInner, spec: RequestSpec) -> Result<SseStream<T>>
where
    T: DeserializeOwned + Send + 'static,
{
    let (response, attempts) = execute_response(inner, spec).await?;
    let meta = build_response_meta(&response, inner.provider.kind(), attempts);
    Ok(RawSseStream::new(response, meta).into_typed())
}

/// 发送 SSE 请求并返回原始事件流。
#[instrument(skip(inner, spec), fields(endpoint_id = spec.endpoint_id, provider = ?inner.provider.kind()))]
#[allow(dead_code)]
pub(crate) async fn execute_raw_sse(
    inner: &ClientInner,
    spec: RequestSpec,
) -> Result<RawSseStream> {
    let (response, attempts) = execute_response(inner, spec).await?;
    let meta = build_response_meta(&response, inner.provider.kind(), attempts);
    Ok(RawSseStream::new(response, meta))
}

/// 发送请求并返回标准 `http::Response<Bytes>`。
#[instrument(skip(inner, spec))]
pub(crate) async fn execute_raw_http(
    inner: &ClientInner,
    spec: RequestSpec,
) -> Result<http::Response<Bytes>> {
    let response = execute_bytes(inner, spec).await?;
    Ok(into_http_response(&response.meta, response.data))
}

#[instrument(skip(inner, spec), fields(endpoint_id = spec.endpoint_id, provider = ?inner.provider.kind()))]
async fn execute(inner: &ClientInner, spec: RequestSpec) -> Result<(Bytes, ResponseMeta)> {
    let (response, attempts) = execute_response(inner, spec).await?;
    let meta = build_response_meta(&response, inner.provider.kind(), attempts);
    let bytes = response
        .bytes()
        .await
        .map_err(|error| Error::Connection(ConnectionError::new(error.to_string())))?;
    Ok((bytes, meta))
}

#[instrument(skip(inner, spec), fields(endpoint_id = spec.endpoint_id, provider = ?inner.provider.kind()))]
async fn execute_response(
    inner: &ClientInner,
    spec: RequestSpec,
) -> Result<(reqwest::Response, usize)> {
    let max_retries = spec
        .options
        .max_retries
        .unwrap_or(inner.options.max_retries);
    let timeout_duration = spec.options.timeout.unwrap_or(inner.options.timeout);
    let cancellation_token = spec.options.cancellation_token.clone();

    let mut attempt = 0u32;
    let mut last_error: Option<Error> = None;

    while attempt <= max_retries {
        if let Some(token) = &cancellation_token
            && token.is_cancelled()
        {
            return Err(Error::Cancelled);
        }

        inner.log(
            LogLevel::Debug,
            "openai_core::transport",
            "发送请求",
            BTreeMap::from([
                ("attempt".into(), attempt.to_string()),
                ("max_retries".into(), max_retries.to_string()),
                ("endpoint_id".into(), spec.endpoint_id.to_string()),
                ("provider".into(), format!("{:?}", inner.provider.kind())),
            ]),
        );
        let request = build_request(inner, &spec).await?;
        let execute_future = inner.http.execute(request);

        let result = if let Some(token) = &cancellation_token {
            tokio::select! {
                biased;
                _ = token.cancelled() => Err(Error::Cancelled),
                response = timeout(timeout_duration, execute_future) => match response {
                    Ok(response) => response.map_err(|error| Error::Connection(ConnectionError::new(error.to_string()))),
                    Err(_) => Err(Error::Timeout),
                }
            }
        } else {
            match timeout(timeout_duration, execute_future).await {
                Ok(response) => response
                    .map_err(|error| Error::Connection(ConnectionError::new(error.to_string()))),
                Err(_) => Err(Error::Timeout),
            }
        };

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok((response, attempt as usize + 1));
                }

                let status = response.status();
                let retry_after = extract_retry_after(response.headers());
                let request_id = extract_request_id(response.headers());
                let body = response.text().await.unwrap_or_default();
                let raw = serde_json::from_str::<Value>(&body)
                    .ok()
                    .map(JsonPayload::from);
                let message = extract_error_message(&raw).unwrap_or_else(|| body.clone());
                let api_error = ApiError::new(
                    status.as_u16(),
                    if message.is_empty() {
                        status.to_string()
                    } else {
                        message
                    },
                    request_id,
                    inner.provider.kind(),
                    raw,
                );
                let error = inner.provider.profile().adapt_error(api_error);

                if (status.as_u16() == 429 || status.is_server_error()) && attempt < max_retries {
                    let delay = retry_after.unwrap_or_else(|| backoff_duration(attempt));
                    inner.log(
                        LogLevel::Info,
                        "openai_core::transport",
                        "请求失败，准备重试",
                        BTreeMap::from([
                            ("attempt".into(), attempt.to_string()),
                            ("delay_ms".into(), delay.as_millis().to_string()),
                            ("status".into(), status.as_u16().to_string()),
                            ("endpoint_id".into(), spec.endpoint_id.to_string()),
                            ("provider".into(), format!("{:?}", inner.provider.kind())),
                        ]),
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    last_error = Some(error);
                    continue;
                }

                return Err(error);
            }
            Err(error) => {
                if matches!(error, Error::Timeout | Error::Connection(_)) && attempt < max_retries {
                    let delay = backoff_duration(attempt);
                    inner.log(
                        LogLevel::Info,
                        "openai_core::transport",
                        "请求执行异常，准备重试",
                        BTreeMap::from([
                            ("attempt".into(), attempt.to_string()),
                            ("delay_ms".into(), delay.as_millis().to_string()),
                            ("endpoint_id".into(), spec.endpoint_id.to_string()),
                            ("provider".into(), format!("{:?}", inner.provider.kind())),
                        ]),
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    last_error = Some(error);
                    continue;
                }
                return Err(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| Error::InvalidConfig("请求执行失败".into())))
}

async fn build_request(inner: &ClientInner, spec: &RequestSpec) -> Result<reqwest::Request> {
    let context = prepare_request_context(
        inner,
        spec.endpoint_id,
        spec.path.clone(),
        spec.body.clone(),
        &spec.options,
    )
    .await?;
    let body = context.body.clone();
    let url = join_url(inner.base_url(), &context.path)?;
    let mut builder = inner.http.request(spec.method.clone(), url);

    for (key, value) in &context.headers {
        builder = builder.header(key, value);
    }
    if !context.query.is_empty() {
        builder = builder.query(&context.query);
    }

    if let Some(multipart) = &spec.multipart {
        let mut form = reqwest::multipart::Form::new();
        let mut fields = Vec::new();
        if let Some(body) = &body {
            flatten_json_to_multipart_fields("", body.as_raw(), &mut fields);
        }
        fields.extend(multipart.fields.iter().cloned());
        for field in &fields {
            form = form.text(field.name.clone(), field.value.clone());
        }
        for (name, source) in &multipart.files {
            form = form.part(name.clone(), source.to_part()?);
        }
        builder = builder.multipart(form);
    } else if let Some(body) = &body {
        builder = builder.json(body.as_raw());
    }

    builder
        .build()
        .map_err(|error| Error::InvalidConfig(format!("构建请求失败: {error}")))
}

/// 按客户端默认配置、Provider 规则和鉴权策略生成最终请求上下文。
pub(crate) async fn prepare_request_context(
    inner: &ClientInner,
    endpoint_id: &'static str,
    path: String,
    body: Option<Value>,
    options: &RequestOptions,
) -> Result<RequestContext> {
    let provider = inner.provider.profile();
    let merged_headers = options.merged_headers(&inner.options.default_headers);
    let merged_query = options.merged_query(&inner.options.default_query);

    let mut context = RequestContext {
        endpoint_id,
        path,
        query: merged_query,
        headers: merged_headers,
        body: body.map(JsonPayload::from),
    };

    provider.validate_request(
        endpoint_id,
        context.body.as_ref().map(JsonPayload::as_raw),
        inner.options.compatibility_mode,
    )?;
    provider.prepare_request(&mut context)?;
    apply_auth(
        &inner.api_key_source,
        inner.provider.profile().auth_scheme(),
        &mut context,
    )
    .await?;
    Ok(context)
}

async fn apply_auth(
    api_key_source: &Option<ApiKeySource>,
    auth_scheme: AuthScheme,
    context: &mut RequestContext,
) -> Result<()> {
    let Some(api_key_source) = api_key_source else {
        return Ok(());
    };
    let api_key = api_key_source.resolve_async().await?;
    let api_key = api_key.expose_secret().to_owned();

    match auth_scheme {
        AuthScheme::Bearer => {
            context
                .headers
                .insert("authorization".into(), format!("Bearer {api_key}"));
        }
        AuthScheme::ApiKeyHeader => {
            context.headers.insert("api-key".into(), api_key);
        }
        AuthScheme::QueryToken => {
            context.query.insert("api_key".into(), api_key);
        }
        AuthScheme::WebSocketSubprotocol => {
            context
                .headers
                .insert("sec-websocket-protocol".into(), api_key);
        }
    }

    Ok(())
}

/// 将基础地址与相对路径拼接为完整 URL。
pub(crate) fn join_url(base_url: &str, path: &str) -> Result<String> {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    let url = format!("{base}/{path}");
    url::Url::parse(&url)
        .map(|value| value.to_string())
        .map_err(|error| Error::InvalidConfig(format!("基础地址无效: {error}")))
}

fn extract_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    if let Some(value) = headers
        .get("retry-after-ms")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        return Some(Duration::from_millis(value));
    }

    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn extract_request_id(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .or_else(|| headers.get("request-id"))
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn extract_error_message(raw: &Option<JsonPayload>) -> Option<String> {
    raw.as_ref().and_then(|value| {
        let value = value.as_raw();
        if let Some(error) = value.get("error") {
            match error {
                Value::Object(map) => {
                    if let Some(message) = map
                        .get("message")
                        .or_else(|| map.get("error"))
                        .or_else(|| map.get("msg"))
                        .or_else(|| map.get("detail"))
                        .and_then(Value::as_str)
                    {
                        return Some(message.to_owned());
                    }
                }
                Value::String(message) => return Some(message.clone()),
                _ => {}
            }
        }

        value
            .get("message")
            .or_else(|| value.get("msg"))
            .or_else(|| value.get("detail"))
            .or_else(|| value.pointer("/base_resp/status_msg"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
}

fn build_response_meta(
    response: &reqwest::Response,
    provider: crate::providers::ProviderKind,
    attempts: usize,
) -> ResponseMeta {
    ResponseMeta {
        status: response.status(),
        headers: response.headers().clone(),
        request_id: extract_request_id(response.headers()),
        provider,
        attempts,
        url: response.url().to_string(),
    }
}

fn backoff_duration(attempt: u32) -> Duration {
    let base = 100u64 * 2u64.saturating_pow(attempt.min(6));
    Duration::from_millis(base + ((attempt as u64 * 17) % 97))
}

/// 递归把 JSON 对象转换成 Multipart 文本字段。
fn flatten_json_to_multipart_fields(prefix: &str, value: &Value, output: &mut Vec<MultipartField>) {
    match value {
        Value::Null => {}
        Value::Bool(value) => output.push(MultipartField {
            name: prefix.to_owned(),
            value: value.to_string(),
        }),
        Value::Number(value) => output.push(MultipartField {
            name: prefix.to_owned(),
            value: value.to_string(),
        }),
        Value::String(value) => output.push(MultipartField {
            name: prefix.to_owned(),
            value: value.clone(),
        }),
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                let key = format!("{prefix}[{index}]");
                flatten_json_to_multipart_fields(&key, value, output);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                let child = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}[{key}]")
                };
                flatten_json_to_multipart_fields(&child, value, output);
            }
        }
    }
}

/// 合并基础请求体与额外字段。
pub fn merge_json_body(
    body: Option<Value>,
    extra_body: &BTreeMap<String, Value>,
    provider_key: &str,
    provider_options: &BTreeMap<String, Value>,
) -> Value {
    let mut merged = match body {
        Some(Value::Object(object)) => object,
        Some(value) => {
            let mut object = Map::new();
            object.insert("value".into(), value);
            object
        }
        None => Map::new(),
    };

    for (key, value) in extra_body {
        merged.insert(key.clone(), value.clone());
    }

    if !provider_options.is_empty() {
        let provider_options_value = merged
            .entry("provider_options")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(root) = provider_options_value {
            let entry = root
                .entry(provider_key.to_owned())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(provider_root) = entry {
                for (key, value) in provider_options {
                    provider_root.insert(key.clone(), value.clone());
                }
            }
        }
    }

    Value::Object(merged)
}

#[cfg(test)]
mod tests {
    use super::{extract_error_message, flatten_json_to_multipart_fields, merge_json_body};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn test_should_serialize_nested_form_fields() {
        let mut fields = Vec::new();
        let value = json!({
            "purpose": "assistants",
            "metadata": {
                "tags": ["a", "b"],
            }
        });
        flatten_json_to_multipart_fields("", &value, &mut fields);

        assert!(
            fields
                .iter()
                .any(|field| field.name == "purpose" && field.value == "assistants")
        );
        assert!(
            fields
                .iter()
                .any(|field| field.name == "metadata[tags][0]" && field.value == "a")
        );
        assert!(
            fields
                .iter()
                .any(|field| field.name == "metadata[tags][1]" && field.value == "b")
        );
    }

    #[test]
    fn test_should_merge_provider_options_into_body() {
        let body = json!({"model": "gpt-5"});
        let mut extra_body = BTreeMap::new();
        extra_body.insert("thinking".into(), json!({"type": "enabled"}));
        let mut provider_options = BTreeMap::new();
        provider_options.insert("reasoning_split".into(), json!(true));

        let merged = merge_json_body(Some(body), &extra_body, "minimax", &provider_options);

        assert_eq!(merged["thinking"]["type"], "enabled");
        assert_eq!(
            merged["provider_options"]["minimax"]["reasoning_split"],
            true
        );
    }

    #[test]
    fn test_should_extract_top_level_error_string_message() {
        let raw = Some(
            json!({
                "timestamp": "2026-04-06T14:04:49.360+00:00",
                "status": 404,
                "error": "Not Found",
                "path": "/v4/responses"
            })
            .into(),
        );

        assert_eq!(extract_error_message(&raw).as_deref(), Some("Not Found"));
    }

    #[test]
    fn test_should_extract_minimax_style_status_message() {
        let raw = Some(
            json!({
                "base_resp": {
                    "status_code": 429,
                    "status_msg": "Too many requests"
                }
            })
            .into(),
        );

        assert_eq!(
            extract_error_message(&raw).as_deref(),
            Some("Too many requests")
        );
    }
}

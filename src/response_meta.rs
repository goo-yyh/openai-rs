//! 响应元数据定义。

use std::ops::{Deref, DerefMut};

use bytes::Bytes;
use http::{HeaderMap, StatusCode};

use crate::providers::ProviderKind;

/// 表示一次响应携带的元信息。
#[derive(Debug, Clone)]
pub struct ResponseMeta {
    /// HTTP 状态码。
    pub status: StatusCode,
    /// 响应头。
    pub headers: HeaderMap,
    /// 请求 ID。
    pub request_id: Option<String>,
    /// Provider 类型。
    pub provider: ProviderKind,
    /// 实际尝试次数。
    pub attempts: usize,
    /// 最终命中的 URL。
    pub url: String,
}

/// 表示带有元信息的响应对象。
#[derive(Debug, Clone)]
pub struct ApiResponse<T> {
    /// 反序列化后的数据对象。
    pub data: T,
    /// 附带的响应元数据。
    pub meta: ResponseMeta,
}

impl<T> ApiResponse<T> {
    /// 创建新的带元信息响应。
    pub fn new(data: T, meta: ResponseMeta) -> Self {
        Self { data, meta }
    }

    /// 把响应拆分成数据与元信息。
    pub fn into_parts(self) -> (T, ResponseMeta) {
        (self.data, self.meta)
    }
}

impl<T> Deref for ApiResponse<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for ApiResponse<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// 把字节响应转换为标准的 `http::Response<Bytes>`。
pub fn into_http_response(meta: &ResponseMeta, body: Bytes) -> http::Response<Bytes> {
    let mut response = http::Response::builder().status(meta.status);

    if let Some(headers) = response.headers_mut() {
        headers.extend(meta.headers.clone());
    }

    response
        .body(body)
        .unwrap_or_else(|_| http::Response::new(Bytes::new()))
}

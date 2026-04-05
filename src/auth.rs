//! 认证相关的通用抽象。

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use secrecy::{ExposeSecret, SecretString};

use crate::error::{Error, Result};

/// 表示一个可动态生成 API Key 的回调。
pub type ApiKeyProvider = dyn Fn() -> Result<SecretString> + Send + Sync;

/// 表示一个可异步生成 API Key 的回调。
pub type AsyncApiKeyProvider =
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<SecretString>> + Send>> + Send + Sync;

/// 表示客户端使用的 API Key 来源。
#[derive(Clone)]
pub enum ApiKeySource {
    /// 使用固定字符串作为 API Key。
    Static(SecretString),
    /// 每次请求或重试时动态生成 API Key。
    Dynamic(Arc<ApiKeyProvider>),
    /// 每次请求或重试时异步生成 API Key。
    AsyncDynamic(Arc<AsyncApiKeyProvider>),
}

impl ApiKeySource {
    /// 创建一个静态 API Key 来源。
    pub fn from_static<T>(value: T) -> Self
    where
        T: Into<String>,
    {
        Self::Static(SecretString::new(value.into().into()))
    }

    /// 创建一个动态 API Key 来源。
    pub fn from_provider<F>(provider: F) -> Self
    where
        F: Fn() -> Result<SecretString> + Send + Sync + 'static,
    {
        Self::Dynamic(Arc::new(provider))
    }

    /// 创建一个异步 API Key 来源。
    pub fn from_async_provider<F, Fut>(provider: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<SecretString>> + Send + 'static,
    {
        Self::AsyncDynamic(Arc::new(move || Box::pin(provider())))
    }

    /// 在当前时刻解析出可用的 API Key。
    ///
    /// # Errors
    ///
    /// 当动态回调返回错误时返回对应错误。
    ///
    /// 若来源是异步回调，请改用 [`Self::resolve_async`]。
    pub fn resolve(&self) -> Result<SecretString> {
        match self {
            Self::Static(value) => Ok(value.clone()),
            Self::Dynamic(provider) => provider(),
            Self::AsyncDynamic(_) => Err(Error::InvalidConfig(
                "当前 API Key 来源为异步回调，请使用 resolve_async".into(),
            )),
        }
    }

    /// 在当前时刻异步解析出可用的 API Key。
    ///
    /// # Errors
    ///
    /// 当动态回调返回错误时返回对应错误。
    pub async fn resolve_async(&self) -> Result<SecretString> {
        match self {
            Self::Static(value) => Ok(value.clone()),
            Self::Dynamic(provider) => provider(),
            Self::AsyncDynamic(provider) => provider().await,
        }
    }

    /// 返回一个可用于日志的脱敏字符串。
    pub fn redacted(&self) -> String {
        match self {
            Self::Static(secret) => redact_secret(secret.expose_secret()),
            Self::Dynamic(_) => "<dynamic-api-key-provider>".into(),
            Self::AsyncDynamic(_) => "<async-api-key-provider>".into(),
        }
    }
}

impl fmt::Debug for ApiKeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ApiKeySource")
            .field(&self.redacted())
            .finish()
    }
}

fn redact_secret(secret: &str) -> String {
    if secret.is_empty() {
        return "<empty-secret>".into();
    }

    if secret.len() <= 8 {
        return "********".into();
    }

    let prefix = &secret[..4];
    let suffix = &secret[secret.len() - 4..];
    format!("{prefix}****{suffix}")
}

impl From<SecretString> for ApiKeySource {
    fn from(value: SecretString) -> Self {
        Self::Static(value)
    }
}

impl TryFrom<Option<ApiKeySource>> for ApiKeySource {
    type Error = Error;

    fn try_from(value: Option<ApiKeySource>) -> Result<Self> {
        value.ok_or(Error::MissingCredentials)
    }
}

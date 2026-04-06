//! 客户端入口与构建器。

use std::collections::BTreeMap;
use std::env;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use secrecy::SecretString;
use tracing::{debug, error, info, warn};

use crate::auth::ApiKeySource;
use crate::config::{ClientOptions, LogLevel, LogRecord, Logger, LoggerHandle};
use crate::error::{Error, Result};
use crate::pagination::{CursorPage, ListEnvelope};
use crate::providers::{AzureOptions, CompatibilityMode, Provider, ProviderKind};
use crate::resources::{
    AudioResource, BatchesResource, BetaResource, ChatResource, CompletionsResource,
    ContainersResource, ConversationsResource, EmbeddingsResource, EvalsResource, FilesResource,
    FineTuningResource, GradersResource, ImagesResource, ModelsResource, ModerationsResource,
    RealtimeResource, ResponsesResource, SkillsResource, UploadsResource, VectorStoresResource,
    VideosResource, WebhooksResource,
};
use crate::transport::{
    RequestSpec, execute_bytes, execute_json, execute_raw_http, execute_raw_sse, execute_sse,
};
use crate::{ApiResponse, RawSseStream, SseStream};

/// `Client` 是对底层 HTTP 客户端的轻量封装。
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

/// 客户端内部共享状态。
#[derive(Debug)]
pub(crate) struct ClientInner {
    pub(crate) http: reqwest::Client,
    pub(crate) options: ClientOptions,
    pub(crate) api_key_source: Option<ApiKeySource>,
    pub(crate) provider: Provider,
}

/// 表示分页下一页请求所需的元信息。
#[derive(Debug, Clone)]
pub struct PageRequestSpec {
    /// 发起下一页请求的客户端。
    pub client: Client,
    /// 端点 ID。
    pub endpoint_id: &'static str,
    /// HTTP 方法。
    pub method: http::Method,
    /// 请求路径。
    pub path: String,
    /// 查询参数。
    pub query: BTreeMap<String, Option<String>>,
}

/// `Client` 的构建器。
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    options: ClientOptions,
    api_key_source: Option<ApiKeySource>,
    azure_options: AzureOptions,
    azure_endpoint: Option<String>,
    azure_configured: bool,
    http_client: Option<reqwest::Client>,
}

impl Client {
    /// 创建客户端构建器。
    pub fn builder() -> ClientBuilder {
        ClientBuilder::from_env()
    }

    /// 返回当前客户端的 Provider。
    pub fn provider(&self) -> &Provider {
        &self.inner.provider
    }

    /// 返回当前客户端的基础地址。
    pub fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    /// 使用闭包覆盖一部分客户端选项，并返回新客户端。
    pub fn with_options<F>(&self, mutate: F) -> Self
    where
        F: FnOnce(&mut ClientOptions),
    {
        let mut options = self.inner.options.clone();
        mutate(&mut options);
        Self::from_parts(
            self.inner.http.clone(),
            options.provider.clone(),
            self.inner.api_key_source.clone(),
            options,
        )
    }

    pub(crate) fn from_parts(
        http: reqwest::Client,
        provider: Provider,
        api_key_source: Option<ApiKeySource>,
        mut options: ClientOptions,
    ) -> Self {
        options.provider = provider.clone();
        Self {
            inner: Arc::new(ClientInner {
                http,
                options,
                api_key_source,
                provider,
            }),
        }
    }

    pub(crate) async fn execute_json<T>(&self, spec: RequestSpec) -> Result<ApiResponse<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        execute_json(&self.inner, spec).await
    }

    pub(crate) async fn execute_bytes(
        &self,
        spec: RequestSpec,
    ) -> Result<ApiResponse<bytes::Bytes>> {
        execute_bytes(&self.inner, spec).await
    }

    pub(crate) async fn execute_sse<T>(&self, spec: RequestSpec) -> Result<SseStream<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        execute_sse(&self.inner, spec).await
    }

    #[allow(dead_code)]
    pub(crate) async fn execute_raw_sse(&self, spec: RequestSpec) -> Result<RawSseStream> {
        execute_raw_sse(&self.inner, spec).await
    }

    pub(crate) async fn execute_raw_http(
        &self,
        spec: RequestSpec,
    ) -> Result<http::Response<bytes::Bytes>> {
        execute_raw_http(&self.inner, spec).await
    }

    pub(crate) async fn fetch_cursor_page<T>(&self, page: PageRequestSpec) -> Result<CursorPage<T>>
    where
        T: Clone + Send + Sync + serde::de::DeserializeOwned + 'static,
    {
        let method = page.method.clone();
        let mut spec = RequestSpec::new(page.endpoint_id, method.clone(), page.path.clone());
        spec.options.extra_query = page.query;

        let response = self.execute_json::<ListEnvelope<T>>(spec).await?;
        let ListEnvelope {
            object,
            data,
            first_id,
            last_id,
            has_more,
            extra,
        } = response.data;
        let next_query = last_id
            .as_ref()
            .map(|last_id| {
                let mut query = BTreeMap::new();
                query.insert("after".into(), Some(last_id.clone()));
                query
            })
            .unwrap_or_default();
        let page_value = CursorPage::from(ListEnvelope {
            object,
            data,
            first_id,
            last_id,
            has_more,
            extra,
        });
        Ok(page_value.with_next_request(if has_more {
            Some(PageRequestSpec {
                client: self.clone(),
                endpoint_id: page.endpoint_id,
                method,
                path: page.path,
                query: next_query,
            })
        } else {
            None
        }))
    }

    /// 返回顶层 completions 资源。
    pub fn completions(&self) -> CompletionsResource {
        CompletionsResource::new(self.clone())
    }

    /// 返回 chat 命名空间。
    pub fn chat(&self) -> ChatResource {
        ChatResource::new(self.clone())
    }

    /// 返回 embeddings 资源。
    pub fn embeddings(&self) -> EmbeddingsResource {
        EmbeddingsResource::new(self.clone())
    }

    /// 返回 files 资源。
    pub fn files(&self) -> FilesResource {
        FilesResource::new(self.clone())
    }

    /// 返回 images 资源。
    pub fn images(&self) -> ImagesResource {
        ImagesResource::new(self.clone())
    }

    /// 返回 audio 命名空间。
    pub fn audio(&self) -> AudioResource {
        AudioResource::new(self.clone())
    }

    /// 返回 moderations 资源。
    pub fn moderations(&self) -> ModerationsResource {
        ModerationsResource::new(self.clone())
    }

    /// 返回 models 资源。
    pub fn models(&self) -> ModelsResource {
        ModelsResource::new(self.clone())
    }

    /// 返回 fine_tuning 命名空间。
    pub fn fine_tuning(&self) -> FineTuningResource {
        FineTuningResource::new(self.clone())
    }

    /// 返回 graders 命名空间。
    pub fn graders(&self) -> GradersResource {
        GradersResource::new(self.clone())
    }

    /// 返回 vector_stores 资源。
    pub fn vector_stores(&self) -> VectorStoresResource {
        VectorStoresResource::new(self.clone())
    }

    /// 返回 webhooks 资源。
    pub fn webhooks(&self) -> WebhooksResource {
        WebhooksResource::new(self.clone())
    }

    /// 返回 batches 资源。
    pub fn batches(&self) -> BatchesResource {
        BatchesResource::new(self.clone())
    }

    /// 返回 uploads 资源。
    pub fn uploads(&self) -> UploadsResource {
        UploadsResource::new(self.clone())
    }

    /// 返回 responses 资源。
    pub fn responses(&self) -> ResponsesResource {
        ResponsesResource::new(self.clone())
    }

    /// 返回 realtime 资源。
    pub fn realtime(&self) -> RealtimeResource {
        RealtimeResource::new(self.clone())
    }

    /// 返回 conversations 资源。
    pub fn conversations(&self) -> ConversationsResource {
        ConversationsResource::new(self.clone())
    }

    /// 返回 evals 资源。
    pub fn evals(&self) -> EvalsResource {
        EvalsResource::new(self.clone())
    }

    /// 返回 containers 资源。
    pub fn containers(&self) -> ContainersResource {
        ContainersResource::new(self.clone())
    }

    /// 返回 skills 资源。
    pub fn skills(&self) -> SkillsResource {
        SkillsResource::new(self.clone())
    }

    /// 返回 videos 资源。
    pub fn videos(&self) -> VideosResource {
        VideosResource::new(self.clone())
    }

    /// 返回 beta 命名空间。
    pub fn beta(&self) -> BetaResource {
        BetaResource::new(self.clone())
    }
}

impl ClientInner {
    pub(crate) fn base_url(&self) -> &str {
        self.options
            .base_url
            .as_deref()
            .unwrap_or_else(|| self.provider.default_base_url())
    }

    pub(crate) fn log(
        &self,
        level: LogLevel,
        target: &'static str,
        message: impl Into<String>,
        fields: BTreeMap<String, String>,
    ) {
        if !self.options.log_level.allows(level) {
            return;
        }

        let record = LogRecord {
            level,
            target,
            message: message.into(),
            fields,
        };

        if let Some(logger) = &self.options.logger {
            logger.log(&record);
        }

        let rendered_fields = if record.fields.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                record
                    .fields
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        let rendered = format!("[{}] {}{}", target, record.message, rendered_fields);
        match level {
            LogLevel::Off => {}
            LogLevel::Error => error!("{rendered}"),
            LogLevel::Warn => warn!("{rendered}"),
            LogLevel::Info => info!("{rendered}"),
            LogLevel::Debug => debug!("{rendered}"),
        }
    }
}

impl ClientBuilder {
    /// 从环境变量构建默认配置。
    pub fn from_env() -> Self {
        let mut builder = Self::default();

        if let Some(webhook_secret) = read_env("OPENAI_WEBHOOK_SECRET") {
            builder.options.webhook_secret = Some(SecretString::new(webhook_secret.into()));
        }
        if let Some(log_level) =
            read_env("OPENAI_LOG").and_then(|value| value.parse::<LogLevel>().ok())
        {
            builder.options.log_level = log_level;
        }

        if let Some(azure_endpoint) = read_env("AZURE_OPENAI_ENDPOINT") {
            builder = builder.azure_endpoint(azure_endpoint);
            if let Some(api_version) = read_env("OPENAI_API_VERSION") {
                builder = builder.azure_api_version(api_version);
            }
            if let Some(api_key) = read_env("AZURE_OPENAI_API_KEY") {
                builder = builder.api_key(api_key);
            }
            return builder;
        }

        if let Some(base_url) = read_env("OPENAI_BASE_URL") {
            builder.options.base_url = Some(base_url);
        }
        if let Some(api_key) = read_env("OPENAI_API_KEY") {
            builder.api_key_source = Some(ApiKeySource::from_static(api_key));
        }

        builder
    }

    /// 设置 Provider。
    pub fn provider(mut self, provider: Provider) -> Self {
        if provider.kind() != ProviderKind::Azure {
            self.azure_options = AzureOptions::default();
            self.azure_endpoint = None;
            self.azure_configured = false;
        }
        self.options.provider = provider;
        self
    }

    /// 注入一个自定义 `reqwest::Client`。
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// 设置 SDK 内部日志级别。
    pub fn log_level(mut self, log_level: LogLevel) -> Self {
        self.options.log_level = log_level;
        self
    }

    /// 注入一个用户自定义日志器。
    pub fn logger<L>(mut self, logger: L) -> Self
    where
        L: Logger + 'static,
    {
        self.options.logger = Some(LoggerHandle::new(logger));
        self
    }

    /// 设置静态 API Key。
    pub fn api_key<T>(mut self, api_key: T) -> Self
    where
        T: Into<String>,
    {
        self.api_key_source = Some(ApiKeySource::from_static(api_key));
        self
    }

    /// 设置动态 API Key 回调。
    pub fn api_key_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn() -> Result<SecretString> + Send + Sync + 'static,
    {
        self.api_key_source = Some(ApiKeySource::from_provider(provider));
        self
    }

    /// 设置异步 API Key 回调。
    pub fn api_key_async_provider<F, Fut>(mut self, provider: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<SecretString>> + Send + 'static,
    {
        self.api_key_source = Some(ApiKeySource::from_async_provider(provider));
        self
    }

    /// 覆盖基础地址。
    pub fn base_url<T>(mut self, base_url: T) -> Self
    where
        T: Into<String>,
    {
        self.options.base_url = Some(base_url.into());
        self
    }

    /// 设置 Azure 资源级 endpoint。
    ///
    /// 该值应类似 `https://example-resource.openai.azure.com`，
    /// SDK 会在发送请求时自动补上 `/openai`。
    pub fn azure_endpoint<T>(mut self, endpoint: T) -> Self
    where
        T: Into<String>,
    {
        self.azure_endpoint = Some(endpoint.into());
        self.azure_configured = true;
        self.options.provider = Provider::azure_with_options(self.azure_options.clone());
        self
    }

    /// 设置 Azure `api-version`。
    pub fn azure_api_version<T>(mut self, api_version: T) -> Self
    where
        T: Into<String>,
    {
        self.azure_options.api_version = Some(api_version.into());
        self.azure_configured = true;
        self.options.provider = Provider::azure_with_options(self.azure_options.clone());
        self
    }

    /// 设置 Azure 默认 deployment。
    pub fn azure_deployment<T>(mut self, deployment: T) -> Self
    where
        T: Into<String>,
    {
        self.azure_options.deployment = Some(deployment.into());
        self.azure_configured = true;
        self.options.provider = Provider::azure_with_options(self.azure_options.clone());
        self
    }

    /// 切换 Azure 为 Bearer Token 认证。
    pub fn azure_bearer_auth(mut self) -> Self {
        self.azure_options = self.azure_options.bearer_auth();
        self.azure_configured = true;
        self.options.provider = Provider::azure_with_options(self.azure_options.clone());
        self
    }

    /// 设置 Azure AD Bearer Token。
    pub fn azure_ad_token<T>(mut self, token: T) -> Self
    where
        T: Into<String>,
    {
        self.azure_options = self.azure_options.bearer_auth();
        self.azure_configured = true;
        self.options.provider = Provider::azure_with_options(self.azure_options.clone());
        self.api_key_source = Some(ApiKeySource::from_static(token));
        self
    }

    /// 设置 Azure AD Bearer Token 异步提供器。
    pub fn azure_ad_token_provider<F, Fut>(mut self, provider: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<SecretString>> + Send + 'static,
    {
        self.azure_options = self.azure_options.bearer_auth();
        self.azure_configured = true;
        self.options.provider = Provider::azure_with_options(self.azure_options.clone());
        self.api_key_source = Some(ApiKeySource::from_async_provider(provider));
        self
    }

    /// 覆盖默认超时时间。
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = timeout;
        self
    }

    /// 覆盖默认最大重试次数。
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.options.max_retries = max_retries;
        self
    }

    /// 添加默认请求头。
    pub fn default_header<T, U>(mut self, key: T, value: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        self.options
            .default_headers
            .insert(key.into(), value.into());
        self
    }

    /// 批量设置默认请求头。
    pub fn default_headers(mut self, headers: BTreeMap<String, String>) -> Self {
        self.options.default_headers = headers;
        self
    }

    /// 添加默认查询参数。
    pub fn default_query<T, U>(mut self, key: T, value: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        self.options.default_query.insert(key.into(), value.into());
        self
    }

    /// 批量设置默认查询参数。
    pub fn default_query_map(mut self, query: BTreeMap<String, String>) -> Self {
        self.options.default_query = query;
        self
    }

    /// 设置 Webhook 密钥。
    pub fn webhook_secret<T>(mut self, secret: T) -> Self
    where
        T: Into<String>,
    {
        self.options.webhook_secret = Some(SecretString::new(secret.into().into()));
        self
    }

    /// 设置兼容性模式。
    pub fn compatibility_mode(mut self, mode: CompatibilityMode) -> Self {
        self.options.compatibility_mode = mode;
        self
    }

    /// 构建客户端。
    ///
    /// # Errors
    ///
    /// 当基础地址非法或底层 `reqwest::Client` 初始化失败时返回错误。
    pub fn build(self) -> Result<Client> {
        let mut options = self.options;
        if options.provider.kind() == ProviderKind::Azure
            && (self.azure_configured || self.azure_endpoint.is_some())
        {
            options.provider = Provider::azure_with_options(self.azure_options.clone());
            if let Some(endpoint) = self.azure_endpoint {
                if options.base_url.is_some() {
                    return Err(Error::InvalidConfig(
                        "`base_url` 和 `azure_endpoint` 不能同时设置".into(),
                    ));
                }
                options.base_url = Some(endpoint);
            }
        }

        let http = if let Some(client) = self.http_client {
            client
        } else {
            let mut default_headers = reqwest::header::HeaderMap::new();
            default_headers.insert(
                reqwest::header::USER_AGENT,
                reqwest::header::HeaderValue::from_static("openai-rs/0.1.0"),
            );

            let mut builder = reqwest::Client::builder().default_headers(default_headers);
            if should_disable_proxy_for_base_url(options.base_url.as_deref()) {
                builder = builder.no_proxy();
            }

            builder
                .build()
                .map_err(|error| Error::InvalidConfig(format!("创建 HTTP 客户端失败: {error}")))?
        };
        Ok(Client::from_parts(
            http,
            options.provider.clone(),
            self.api_key_source,
            options,
        ))
    }
}

fn read_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn should_disable_proxy_for_base_url(base_url: Option<&str>) -> bool {
    let Some(base_url) = base_url else {
        return false;
    };

    let Ok(url) = url::Url::parse(base_url) else {
        return false;
    };

    matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("[::1]") | Some("::1")
    )
}

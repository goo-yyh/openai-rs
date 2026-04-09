//! Provider 兼容层。

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, ProviderCompatibilityError, Result};
use crate::json_payload::JsonPayload;

/// 表示支持的 Provider 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// 官方 OpenAI Provider。
    OpenAI,
    /// Azure OpenAI Provider。
    Azure,
    /// 智谱兼容 Provider。
    Zhipu,
    /// MiniMax 兼容 Provider。
    MiniMax,
    /// ZenMux 兼容 Provider。
    ZenMux,
    /// 自定义 Provider。
    Custom,
}

impl ProviderKind {
    /// 返回 provider 对应的小写键。
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Azure => "azure",
            Self::Zhipu => "zhipu",
            Self::MiniMax => "minimax",
            Self::ZenMux => "zenmux",
            Self::Custom => "custom",
        }
    }
}

/// 表示 Provider 的认证方案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthScheme {
    /// 使用 `Authorization: Bearer <token>`。
    Bearer,
    /// 使用 `api-key: <token>`。
    ApiKeyHeader,
    /// 使用查询参数传递令牌。
    QueryToken,
    /// 使用 WebSocket 子协议传递令牌。
    WebSocketSubprotocol,
}

/// 表示兼容性校验模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityMode {
    /// 尽可能透传未知字段。
    Passthrough,
    /// 对已知风险发出警告。
    Warn,
    /// 对已知不兼容字段直接报错。
    Strict,
}

/// 表示 Azure OpenAI 的认证模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AzureAuthMode {
    /// 使用 `api-key` 请求头。
    #[default]
    ApiKey,
    /// 使用 `Authorization: Bearer <token>`。
    Bearer,
}

impl AzureAuthMode {
    /// 转换为底层通用认证方案。
    pub fn auth_scheme(self) -> AuthScheme {
        match self {
            Self::ApiKey => AuthScheme::ApiKeyHeader,
            Self::Bearer => AuthScheme::Bearer,
        }
    }
}

/// 表示 Azure Provider 的可配置选项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AzureOptions {
    /// Azure OpenAI `api-version`。
    pub api_version: Option<String>,
    /// 默认 deployment 名称。
    pub deployment: Option<String>,
    /// Azure 认证模式。
    #[serde(default)]
    pub auth_mode: AzureAuthMode,
}

impl AzureOptions {
    /// 创建默认 Azure 选项。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 `api-version`。
    pub fn api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = Some(api_version.into());
        self
    }

    /// 设置默认 deployment。
    pub fn deployment(mut self, deployment: impl Into<String>) -> Self {
        self.deployment = Some(deployment.into());
        self
    }

    /// 切换为 Bearer Token 认证。
    pub fn bearer_auth(mut self) -> Self {
        self.auth_mode = AzureAuthMode::Bearer;
        self
    }

    /// 切换为 `api-key` 认证。
    pub fn api_key_auth(mut self) -> Self {
        self.auth_mode = AzureAuthMode::ApiKey;
        self
    }
}

/// 表示 Provider 的能力集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet {
    /// 是否支持聊天补全。
    pub chat_completions: bool,
    /// 是否支持 Responses API。
    pub responses: bool,
    /// 是否支持模型列表。
    pub models: bool,
    /// 是否支持 SSE 流。
    pub streaming: bool,
    /// 是否支持工具调用。
    pub tools: bool,
    /// 是否支持 Webhook。
    pub webhooks: bool,
}

const FULL_CAPABILITIES: CapabilitySet = CapabilitySet {
    chat_completions: true,
    responses: true,
    models: true,
    streaming: true,
    tools: true,
    webhooks: true,
};

const CHAT_ONLY_CAPABILITIES: CapabilitySet = CapabilitySet {
    chat_completions: true,
    responses: false,
    models: true,
    streaming: true,
    tools: true,
    webhooks: false,
};

/// 表示 Provider 在发送请求前可修改的上下文。
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// 逻辑端点 ID。
    pub endpoint_id: &'static str,
    /// HTTP 路径。
    pub path: String,
    /// 查询参数。
    pub query: BTreeMap<String, String>,
    /// 请求头。
    pub headers: BTreeMap<String, String>,
    /// JSON 请求体。
    pub body: Option<JsonPayload>,
}

/// ProviderProfile 用于屏蔽不同兼容 Provider 的差异。
pub trait ProviderProfile: Send + Sync {
    /// 返回 Provider 类型。
    fn kind(&self) -> ProviderKind;
    /// 返回默认基础地址。
    fn default_base_url(&self) -> &str;
    /// 返回认证方案。
    fn auth_scheme(&self) -> AuthScheme;
    /// 返回能力集合。
    fn capabilities(&self) -> &'static CapabilitySet;
    /// 在请求真正构建前对请求做进一步调整。
    fn prepare_request(&self, ctx: &mut RequestContext) -> Result<()>;
    /// 根据 Provider 规则适配错误。
    fn adapt_error(&self, error: crate::ApiError) -> Error {
        Error::Api(error)
    }
    /// 在发送前校验请求是否符合当前 Provider 要求。
    fn validate_request(
        &self,
        endpoint_id: &'static str,
        body: Option<&Value>,
        mode: CompatibilityMode,
    ) -> Result<()>;
}

/// 对外暴露的 Provider 句柄。
#[derive(Clone)]
pub struct Provider {
    inner: Arc<dyn ProviderProfile>,
}

impl fmt::Debug for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Provider")
            .field("kind", &self.kind())
            .field("default_base_url", &self.default_base_url())
            .finish()
    }
}

impl Provider {
    /// 创建 OpenAI Provider。
    pub fn openai() -> Self {
        Self {
            inner: Arc::new(OpenAiProfile),
        }
    }

    /// 创建 Azure Provider。
    pub fn azure() -> Self {
        Self::azure_with_options(AzureOptions::default())
    }

    /// 创建带自定义选项的 Azure Provider。
    pub fn azure_with_options(options: AzureOptions) -> Self {
        Self {
            inner: Arc::new(AzureProfile::new(options)),
        }
    }

    /// 创建智谱 Provider。
    pub fn zhipu() -> Self {
        Self {
            inner: Arc::new(ZhipuProfile),
        }
    }

    /// 创建 MiniMax Provider。
    pub fn minimax() -> Self {
        Self {
            inner: Arc::new(MiniMaxProfile),
        }
    }

    /// 创建 ZenMux Provider。
    pub fn zenmux() -> Self {
        Self {
            inner: Arc::new(ZenMuxProfile),
        }
    }

    /// 创建自定义 Provider。
    pub fn custom<T>(profile: T) -> Self
    where
        T: ProviderProfile + 'static,
    {
        Self {
            inner: Arc::new(profile),
        }
    }

    /// 返回 Provider 类型。
    pub fn kind(&self) -> ProviderKind {
        self.inner.kind()
    }

    /// 返回默认基础地址。
    pub fn default_base_url(&self) -> &str {
        self.inner.default_base_url()
    }

    /// 返回 ProviderProfile 引用。
    pub fn profile(&self) -> &(dyn ProviderProfile + Send + Sync) {
        self.inner.as_ref()
    }
}

/// 表示自定义 Provider 实现。
#[derive(Debug, Clone)]
pub struct CustomProfile {
    /// Provider 的自定义名称。
    pub name: String,
    /// 默认基础地址。
    pub base_url: String,
    /// 认证方案。
    pub auth_scheme: AuthScheme,
    /// 能力集合。
    pub capabilities: CapabilitySet,
}

impl ProviderProfile for CustomProfile {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Custom
    }

    fn default_base_url(&self) -> &str {
        &self.base_url
    }

    fn auth_scheme(&self) -> AuthScheme {
        self.auth_scheme
    }

    fn capabilities(&self) -> &'static CapabilitySet {
        Box::leak(Box::new(self.capabilities))
    }

    fn prepare_request(&self, _ctx: &mut RequestContext) -> Result<()> {
        Ok(())
    }

    fn validate_request(
        &self,
        _endpoint_id: &'static str,
        _body: Option<&Value>,
        _mode: CompatibilityMode,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct AzureProfile {
    options: AzureOptions,
}

impl AzureProfile {
    fn new(options: AzureOptions) -> Self {
        Self { options }
    }

    fn api_version(&self) -> &str {
        self.options
            .api_version
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("2025-03-01-preview")
    }

    fn auth_scheme(&self) -> AuthScheme {
        self.options.auth_mode.auth_scheme()
    }

    fn deployment_for(&self, ctx: &RequestContext) -> Option<String> {
        if ctx.endpoint_id == "realtime.ws.connect" {
            return ctx
                .query
                .get("deployment")
                .cloned()
                .or_else(|| self.options.deployment.clone())
                .filter(|value| !value.trim().is_empty());
        }

        if !azure_deployment_path_required(&ctx.path) {
            return None;
        }

        self.options
            .deployment
            .clone()
            .or_else(|| {
                ctx.body
                    .as_ref()
                    .and_then(|value| value.get("model"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|value| !value.trim().is_empty())
    }
}

#[derive(Debug, Clone, Copy)]
struct OpenAiProfile;

#[derive(Debug, Clone, Copy)]
struct ZhipuProfile;

#[derive(Debug, Clone, Copy)]
struct MiniMaxProfile;

#[derive(Debug, Clone, Copy)]
struct ZenMuxProfile;

impl ProviderProfile for OpenAiProfile {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAI
    }

    fn default_base_url(&self) -> &str {
        "https://api.openai.com/v1"
    }

    fn auth_scheme(&self) -> AuthScheme {
        AuthScheme::Bearer
    }

    fn capabilities(&self) -> &'static CapabilitySet {
        &FULL_CAPABILITIES
    }

    fn prepare_request(&self, _ctx: &mut RequestContext) -> Result<()> {
        Ok(())
    }

    fn validate_request(
        &self,
        _endpoint_id: &'static str,
        _body: Option<&Value>,
        _mode: CompatibilityMode,
    ) -> Result<()> {
        Ok(())
    }
}

impl ProviderProfile for AzureProfile {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Azure
    }

    fn default_base_url(&self) -> &str {
        "https://example-resource.openai.azure.com"
    }

    fn auth_scheme(&self) -> AuthScheme {
        self.auth_scheme()
    }

    fn capabilities(&self) -> &'static CapabilitySet {
        &FULL_CAPABILITIES
    }

    fn prepare_request(&self, ctx: &mut RequestContext) -> Result<()> {
        ctx.query
            .entry("api-version".into())
            .or_insert_with(|| self.api_version().into());

        if !ctx.path.starts_with("/openai") {
            ctx.path = format!("/openai{}", ctx.path);
        }

        if let Some(deployment) = self.deployment_for(ctx)
            && ctx.endpoint_id == "realtime.ws.connect"
        {
            ctx.query.insert("deployment".into(), deployment);
        } else if let Some(deployment) = self.deployment_for(ctx)
            && !ctx.path.contains("/deployments/")
        {
            ctx.path =
                ctx.path
                    .replacen("/openai/", &format!("/openai/deployments/{deployment}/"), 1);
        }

        Ok(())
    }

    fn validate_request(
        &self,
        _endpoint_id: &'static str,
        _body: Option<&Value>,
        _mode: CompatibilityMode,
    ) -> Result<()> {
        Ok(())
    }
}

impl ProviderProfile for ZhipuProfile {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Zhipu
    }

    fn default_base_url(&self) -> &str {
        "https://open.bigmodel.cn/api/paas/v4"
    }

    fn auth_scheme(&self) -> AuthScheme {
        AuthScheme::Bearer
    }

    fn capabilities(&self) -> &'static CapabilitySet {
        &CHAT_ONLY_CAPABILITIES
    }

    fn prepare_request(&self, _ctx: &mut RequestContext) -> Result<()> {
        Ok(())
    }

    fn validate_request(
        &self,
        _endpoint_id: &'static str,
        _body: Option<&Value>,
        _mode: CompatibilityMode,
    ) -> Result<()> {
        Ok(())
    }
}

impl ProviderProfile for MiniMaxProfile {
    fn kind(&self) -> ProviderKind {
        ProviderKind::MiniMax
    }

    fn default_base_url(&self) -> &str {
        "https://api.minimaxi.com/v1"
    }

    fn auth_scheme(&self) -> AuthScheme {
        AuthScheme::Bearer
    }

    fn capabilities(&self) -> &'static CapabilitySet {
        &CHAT_ONLY_CAPABILITIES
    }

    fn prepare_request(&self, _ctx: &mut RequestContext) -> Result<()> {
        Ok(())
    }

    fn validate_request(
        &self,
        _endpoint_id: &'static str,
        body: Option<&Value>,
        mode: CompatibilityMode,
    ) -> Result<()> {
        if mode != CompatibilityMode::Strict {
            return Ok(());
        }

        let Some(body) = body else {
            return Ok(());
        };

        if let Some(value) = body.get("n").and_then(Value::as_i64)
            && value != 1
        {
            return Err(ProviderCompatibilityError::new(
                ProviderKind::MiniMax,
                "MiniMax 在严格模式下仅支持 n = 1",
            )
            .into());
        }

        if contains_key(body, "function_call") {
            return Err(ProviderCompatibilityError::new(
                ProviderKind::MiniMax,
                "MiniMax 在严格模式下不再支持旧版 function_call 字段，请改用 tools",
            )
            .into());
        }

        if contains_any_type(body, &["input_image", "image", "input_audio", "audio"]) {
            return Err(ProviderCompatibilityError::new(
                ProviderKind::MiniMax,
                "MiniMax 在严格模式下不支持图像或音频输入",
            )
            .into());
        }

        Ok(())
    }
}

impl ProviderProfile for ZenMuxProfile {
    fn kind(&self) -> ProviderKind {
        ProviderKind::ZenMux
    }

    fn default_base_url(&self) -> &str {
        "https://zenmux.ai/api/v1"
    }

    fn auth_scheme(&self) -> AuthScheme {
        AuthScheme::Bearer
    }

    fn capabilities(&self) -> &'static CapabilitySet {
        &FULL_CAPABILITIES
    }

    fn prepare_request(&self, _ctx: &mut RequestContext) -> Result<()> {
        Ok(())
    }

    fn validate_request(
        &self,
        _endpoint_id: &'static str,
        body: Option<&Value>,
        mode: CompatibilityMode,
    ) -> Result<()> {
        if mode != CompatibilityMode::Strict {
            return Ok(());
        }

        let Some(model) = body
            .and_then(|value| value.get("model"))
            .and_then(Value::as_str)
        else {
            return Ok(());
        };

        if !model.contains('/') || model.starts_with('/') || model.ends_with('/') {
            return Err(ProviderCompatibilityError::new(
                ProviderKind::ZenMux,
                "ZenMux 在严格模式下要求 model 采用 <provider>/<model_name> 形式",
            )
            .into());
        }

        Ok(())
    }
}

fn contains_key(value: &Value, target: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key(target) || map.values().any(|value| contains_key(value, target))
        }
        Value::Array(values) => values.iter().any(|value| contains_key(value, target)),
        _ => false,
    }
}

fn contains_any_type(value: &Value, targets: &[&str]) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, nested)| {
            (key == "type"
                && nested
                    .as_str()
                    .is_some_and(|value| targets.contains(&value)))
                || contains_any_type(nested, targets)
        }),
        Value::Array(values) => values.iter().any(|value| contains_any_type(value, targets)),
        _ => false,
    }
}

fn azure_deployment_path_required(path: &str) -> bool {
    matches!(
        path.trim_end_matches('/'),
        "/completions"
            | "/chat/completions"
            | "/embeddings"
            | "/audio/transcriptions"
            | "/audio/translations"
            | "/audio/speech"
            | "/images/generations"
            | "/images/edits"
            | "/batches"
            | "/openai/completions"
            | "/openai/chat/completions"
            | "/openai/embeddings"
            | "/openai/audio/transcriptions"
            | "/openai/audio/translations"
            | "/openai/audio/speech"
            | "/openai/images/generations"
            | "/openai/images/edits"
            | "/openai/batches"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_use_zhipu_default_base_url() {
        let provider = Provider::zhipu();
        assert_eq!(
            provider.default_base_url(),
            "https://open.bigmodel.cn/api/paas/v4"
        );
    }

    #[test]
    fn test_should_use_minimax_default_base_url() {
        let provider = Provider::minimax();
        assert_eq!(provider.default_base_url(), "https://api.minimaxi.com/v1");
    }

    #[test]
    fn test_should_use_zenmux_default_base_url() {
        let provider = Provider::zenmux();
        assert_eq!(provider.default_base_url(), "https://zenmux.ai/api/v1");
    }

    #[test]
    fn test_should_validate_minimax_n_equals_one_in_strict_mode() {
        let provider = Provider::minimax();
        let body = serde_json::json!({
            "model": "MiniMax-M2.7",
            "messages": [{"role": "user", "content": "hello"}],
            "n": 2
        });
        let error = provider
            .profile()
            .validate_request(
                "chat.completions.create",
                Some(&body),
                CompatibilityMode::Strict,
            )
            .unwrap_err();
        assert!(matches!(error, Error::ProviderCompatibility(_)));
    }

    #[test]
    fn test_should_validate_zenmux_model_id_format_in_strict_mode() {
        let provider = Provider::zenmux();
        let body = serde_json::json!({
            "model": "gpt-5",
            "input": "hello"
        });
        let error = provider
            .profile()
            .validate_request("responses.create", Some(&body), CompatibilityMode::Strict)
            .unwrap_err();
        assert!(matches!(error, Error::ProviderCompatibility(_)));
    }

    #[test]
    fn test_should_preserve_passthrough_mode_for_minimax() {
        let provider = Provider::minimax();
        let body = serde_json::json!({
            "model": "MiniMax-M2.7",
            "messages": [{"role": "user", "content": "hello"}],
            "n": 3
        });
        provider
            .profile()
            .validate_request(
                "chat.completions.create",
                Some(&body),
                CompatibilityMode::Passthrough,
            )
            .unwrap();
    }

    #[test]
    fn test_should_inject_azure_api_version_and_prefix_path() {
        let provider =
            Provider::azure_with_options(AzureOptions::new().api_version("2024-02-15-preview"));
        let mut context = RequestContext {
            endpoint_id: "responses.create",
            path: "/responses".into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            body: None,
        };

        provider.profile().prepare_request(&mut context).unwrap();

        assert_eq!(context.path, "/openai/responses");
        assert_eq!(
            context.query.get("api-version").map(String::as_str),
            Some("2024-02-15-preview")
        );
    }

    #[test]
    fn test_should_preserve_existing_azure_api_version_query() {
        let provider = Provider::azure();
        let mut context = RequestContext {
            endpoint_id: "responses.create",
            path: "/responses".into(),
            query: BTreeMap::from([("api-version".into(), "custom-version".into())]),
            headers: BTreeMap::new(),
            body: None,
        };

        provider.profile().prepare_request(&mut context).unwrap();

        assert_eq!(
            context.query.get("api-version").map(String::as_str),
            Some("custom-version")
        );
    }

    #[test]
    fn test_should_inject_azure_deployment_from_body_model() {
        let provider = Provider::azure();
        let mut context = RequestContext {
            endpoint_id: "chat.completions.create",
            path: "/chat/completions".into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            body: Some(
                serde_json::json!({
                    "model": "gpt-4o-deployment"
                })
                .into(),
            ),
        };

        provider.profile().prepare_request(&mut context).unwrap();

        assert_eq!(
            context.path,
            "/openai/deployments/gpt-4o-deployment/chat/completions"
        );
    }

    #[test]
    fn test_should_inject_azure_realtime_deployment_query() {
        let provider =
            Provider::azure_with_options(AzureOptions::new().deployment("rt-deployment"));
        let mut context = RequestContext {
            endpoint_id: "realtime.ws.connect",
            path: "/realtime".into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            body: None,
        };

        provider.profile().prepare_request(&mut context).unwrap();

        assert_eq!(context.path, "/openai/realtime");
        assert_eq!(
            context.query.get("deployment").map(String::as_str),
            Some("rt-deployment")
        );
    }

    #[test]
    fn test_should_switch_azure_auth_scheme_to_bearer() {
        let provider = Provider::azure_with_options(AzureOptions::new().bearer_auth());
        assert_eq!(provider.profile().auth_scheme(), AuthScheme::Bearer);
    }
}

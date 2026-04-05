//! 客户端与请求级配置。

use std::collections::BTreeMap;
use std::time::Duration;

use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

use crate::providers::{CompatibilityMode, Provider};

/// 表示客户端级别的默认配置。
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// 当前客户端使用的 Provider。
    pub provider: Provider,
    /// 覆盖默认的基础地址。
    pub base_url: Option<String>,
    /// 每次请求默认超时时间。
    pub timeout: Duration,
    /// 默认最大重试次数。
    pub max_retries: u32,
    /// 发送前追加到所有请求中的默认请求头。
    pub default_headers: BTreeMap<String, String>,
    /// 发送前追加到所有请求中的默认查询参数。
    pub default_query: BTreeMap<String, String>,
    /// 可选的 Webhook 密钥。
    pub webhook_secret: Option<SecretString>,
    /// Provider 兼容校验模式。
    pub compatibility_mode: CompatibilityMode,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            provider: Provider::openai(),
            base_url: None,
            timeout: Duration::from_secs(600),
            max_retries: 2,
            default_headers: BTreeMap::new(),
            default_query: BTreeMap::new(),
            webhook_secret: None,
            compatibility_mode: CompatibilityMode::Passthrough,
        }
    }
}

/// 表示单次请求可覆盖的配置。
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// 额外请求头。若值为 `None`，则会移除同名默认请求头。
    pub extra_headers: BTreeMap<String, Option<String>>,
    /// 额外查询参数。若值为 `None`，则会移除同名默认查询参数。
    pub extra_query: BTreeMap<String, Option<String>>,
    /// 覆盖客户端默认超时时间。
    pub timeout: Option<Duration>,
    /// 覆盖客户端默认最大重试次数。
    pub max_retries: Option<u32>,
    /// 可选的取消令牌。
    pub cancellation_token: Option<CancellationToken>,
}

impl RequestOptions {
    /// 追加或覆盖一个请求头。
    pub fn insert_header<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.extra_headers.insert(key.into(), Some(value.into()));
    }

    /// 移除一个请求头。
    pub fn remove_header<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.extra_headers.insert(key.into(), None);
    }

    /// 追加或覆盖一个查询参数。
    pub fn insert_query<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.extra_query.insert(key.into(), Some(value.into()));
    }

    /// 移除一个查询参数。
    pub fn remove_query<K>(&mut self, key: K)
    where
        K: Into<String>,
    {
        self.extra_query.insert(key.into(), None);
    }

    /// 合并客户端默认请求头与请求级请求头。
    pub fn merged_headers(&self, defaults: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        merge_kv_maps(defaults, &self.extra_headers)
    }

    /// 合并客户端默认查询参数与请求级查询参数。
    pub fn merged_query(&self, defaults: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        merge_kv_maps(defaults, &self.extra_query)
    }
}

/// 合并默认键值对与请求级覆盖项。
pub fn merge_kv_maps(
    defaults: &BTreeMap<String, String>,
    overrides: &BTreeMap<String, Option<String>>,
) -> BTreeMap<String, String> {
    let mut merged = defaults.clone();

    for (key, value) in overrides {
        match value {
            Some(value) => {
                merged.insert(key.clone(), value.clone());
            }
            None => {
                merged.remove(key);
            }
        }
    }

    merged
}

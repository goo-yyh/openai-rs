//! 客户端与请求级配置。

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

use crate::providers::{CompatibilityMode, Provider};

/// SDK 日志级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LogLevel {
    /// 关闭 SDK 内部日志。
    Off,
    /// 仅输出错误日志。
    Error,
    /// 输出警告和错误日志。
    #[default]
    Warn,
    /// 输出信息、警告和错误日志。
    Info,
    /// 输出全部调试日志。
    Debug,
}

impl LogLevel {
    /// 判断当前配置是否允许输出指定级别的日志。
    pub fn allows(self, level: Self) -> bool {
        self != Self::Off && level <= self
    }

    /// 返回日志级别的稳定字符串表示。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => Ok(Self::Off),
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            other => Err(format!("不支持的日志级别: {other}")),
        }
    }
}

/// 一条 SDK 日志记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// 日志级别。
    pub level: LogLevel,
    /// 日志目标，一般对应子系统。
    pub target: &'static str,
    /// 人类可读消息。
    pub message: String,
    /// 附加字段。
    pub fields: BTreeMap<String, String>,
}

/// 用户自定义日志接收器。
pub trait Logger: Send + Sync {
    /// 处理一条日志记录。
    fn log(&self, record: &LogRecord);
}

impl<F> Logger for F
where
    F: Fn(&LogRecord) + Send + Sync,
{
    fn log(&self, record: &LogRecord) {
        (self)(record);
    }
}

/// 可克隆的日志器句柄。
#[derive(Clone)]
pub struct LoggerHandle {
    inner: Arc<dyn Logger>,
}

impl LoggerHandle {
    /// 创建新的日志器句柄。
    pub fn new<L>(logger: L) -> Self
    where
        L: Logger + 'static,
    {
        Self {
            inner: Arc::new(logger),
        }
    }

    /// 输出一条日志。
    pub fn log(&self, record: &LogRecord) {
        self.inner.log(record);
    }
}

impl fmt::Debug for LoggerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LoggerHandle(..)")
    }
}

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
    /// SDK 内部日志级别。
    pub log_level: LogLevel,
    /// 可选的用户日志器。
    pub logger: Option<LoggerHandle>,
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
            log_level: LogLevel::Warn,
            logger: None,
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

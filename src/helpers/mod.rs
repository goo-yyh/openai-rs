//! Structured output 与工具调用辅助能力。

#[cfg(feature = "tool-runner")]
use std::collections::BTreeMap;
#[cfg(feature = "tool-runner")]
use std::future::Future;
#[cfg(feature = "tool-runner")]
use std::pin::Pin;
#[cfg(feature = "tool-runner")]
use std::sync::Arc;

use schemars::{JsonSchema, schema_for};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::resources::{ChatCompletion, Response};

/// 返回指定类型对应的 JSON Schema。
pub fn json_schema_for<T>() -> Value
where
    T: JsonSchema,
{
    serde_json::to_value(schema_for!(T)).unwrap_or_else(|_| Value::Object(Default::default()))
}

/// 尝试从文本中提取并解析 JSON。
///
/// 该函数会自动去掉常见的 Markdown 代码块包裹。
///
/// # Errors
///
/// 当 JSON 解析失败时返回错误。
pub fn parse_json_payload<T>(payload: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let trimmed = payload.trim();
    let normalized = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|value| value.trim())
        .and_then(|value| value.strip_suffix("```"))
        .map_or(trimmed, str::trim);

    serde_json::from_str(normalized).map_err(|error| {
        Error::Serialization(crate::SerializationError::new(format!(
            "结构化 JSON 解析失败: {error}"
        )))
    })
}

/// 表示已经解析出结构化对象的聊天补全结果。
#[derive(Debug, Clone)]
pub struct ParsedChatCompletion<T> {
    /// 原始聊天补全结果。
    pub response: ChatCompletion,
    /// 反序列化后的结构化对象。
    pub parsed: T,
}

/// 表示已经解析出结构化对象的 Responses 结果。
#[derive(Debug, Clone)]
pub struct ParsedResponse<T> {
    /// 原始 Responses 结果。
    pub response: Response,
    /// 反序列化后的结构化对象。
    pub parsed: T,
}

/// 工具处理函数的异步返回值类型。
#[cfg(feature = "tool-runner")]
#[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
pub type ToolFuture = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// 表示工具处理器。
#[cfg(feature = "tool-runner")]
#[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
pub trait ToolHandler: Send + Sync {
    /// 执行一个工具调用。
    fn call(&self, arguments: Value) -> ToolFuture;
}

#[cfg(feature = "tool-runner")]
impl<F, Fut> ToolHandler for F
where
    F: Fn(Value) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Value>> + Send + 'static,
{
    fn call(&self, arguments: Value) -> ToolFuture {
        Box::pin((self)(arguments))
    }
}

/// 表示单个工具定义。
#[cfg(feature = "tool-runner")]
#[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
#[derive(Clone)]
pub struct ToolDefinition {
    /// 工具名称。
    pub name: String,
    /// 工具描述。
    pub description: Option<String>,
    /// 工具参数 JSON Schema。
    pub parameters: Value,
    handler: Arc<dyn ToolHandler>,
}

#[cfg(feature = "tool-runner")]
impl std::fmt::Debug for ToolDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDefinition")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .finish()
    }
}

#[cfg(feature = "tool-runner")]
impl ToolDefinition {
    /// 使用显式 JSON Schema 创建工具定义。
    pub fn new<T, U, H>(name: T, description: Option<U>, parameters: Value, handler: H) -> Self
    where
        T: Into<String>,
        U: Into<String>,
        H: ToolHandler + 'static,
    {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            parameters,
            handler: Arc::new(handler),
        }
    }

    /// 使用 `schemars` 自动推导参数 Schema。
    pub fn from_schema<TArgs, T, U, H>(name: T, description: Option<U>, handler: H) -> Self
    where
        TArgs: JsonSchema,
        T: Into<String>,
        U: Into<String>,
        H: ToolHandler + 'static,
    {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            parameters: json_schema_for::<TArgs>(),
            handler: Arc::new(handler),
        }
    }

    /// 调用工具处理器。
    pub async fn invoke(&self, arguments: Value) -> Result<Value> {
        self.handler.call(arguments).await
    }
}

/// 表示工具注册表。
#[cfg(feature = "tool-runner")]
#[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolDefinition>,
}

#[cfg(feature = "tool-runner")]
impl ToolRegistry {
    /// 创建空的工具注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个工具。
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// 查询指定名称的工具。
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// 返回所有工具定义。
    pub fn all(&self) -> impl Iterator<Item = &ToolDefinition> {
        self.tools.values()
    }

    /// 判断注册表是否为空。
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

use std::future::Future;
use std::time::Duration;

use openai_rs::resources::{ChatToolDefinition, ChatToolFunction};
use openai_rs::{ApiError, ChatCompletion, ChatCompletionToolCall, Error, ProviderKind, Result};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// 读取环境变量；缺失时让 live test 直接跳过。
pub fn env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => {
            eprintln!("skip live test because {name} is missing");
            None
        }
    }
}

/// 提取首个 choice 的原始文本。
pub fn first_content(response: &ChatCompletion) -> String {
    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_default()
}

/// 移除常见的推理标签与 markdown 代码块，得到更接近用户可见的文本。
pub fn sanitize_visible_text(text: &str) -> String {
    let mut sanitized = text.to_owned();

    while let Some(start) = sanitized.find("<think>") {
        if let Some(end_rel) = sanitized[start..].find("</think>") {
            let end = start + end_rel + "</think>".len();
            sanitized.replace_range(start..end, "");
        } else {
            sanitized.truncate(start);
            break;
        }
    }

    let trimmed = sanitized.trim();
    let without_fence = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(str::trim)
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);

    without_fence.trim().to_owned()
}

/// 提取首个 choice 的可见文本。
pub fn first_visible_content(response: &ChatCompletion) -> String {
    sanitize_visible_text(&first_content(response))
}

/// 解析“看起来像 JSON”的文本。
pub fn parse_jsonish<T>(text: &str) -> serde_json::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(&sanitize_visible_text(text))
}

/// 要求文本至少包含一个目标关键词。
pub fn assert_contains_any(text: &str, keywords: &[&str]) {
    assert!(
        keywords.iter().any(|keyword| text.contains(keyword)),
        "expected text to contain one of {keywords:?}, got: {text}"
    );
}

/// 对易抖动的线上请求做有限重试，仅对连接问题和超时重试。
pub async fn retry_live<T, F, Fut>(label: &str, attempts: usize, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 1..=attempts {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error @ (Error::Connection(_) | Error::Timeout)) if attempt < attempts => {
                eprintln!("{label} transient failure on attempt {attempt}, retrying: {error}");
                last_error = Some(error);
                tokio::time::sleep(Duration::from_secs(attempt as u64 * 2)).await;
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.expect("retry loop must capture an error"))
}

/// 断言标准化 API 错误形态。
pub fn expect_api_error_shape(error: Error, provider: ProviderKind) -> ApiError {
    match error {
        Error::Api(api) => {
            assert_eq!(api.provider, provider);
            assert!(api.status >= 400, "unexpected status: {}", api.status);
            assert!(
                !api.message.trim().is_empty(),
                "expected non-empty api message"
            );
            api
        }
        other => panic!("expected Api error for {provider:?}, got: {other:?}"),
    }
}

/// 构造一个简单的加法工具定义。
pub fn add_numbers_tool() -> ChatToolDefinition {
    ChatToolDefinition {
        tool_type: "function".into(),
        function: ChatToolFunction {
            name: "add_numbers".into(),
            description: Some("Add two integers and return their sum.".into()),
            parameters: json!({
                "type": "object",
                "properties": {
                    "a": {"type": "integer"},
                    "b": {"type": "integer"}
                },
                "required": ["a", "b"]
            }),
        },
    }
}

/// 构造强制模型选择指定函数工具的参数。
pub fn force_tool_choice(name: &str) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name
        }
    })
}

/// 解析工具调用参数 JSON。
pub fn parse_tool_arguments(tool_call: &ChatCompletionToolCall) -> Value {
    serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|error| {
        panic!(
            "failed to parse tool arguments for {}: {error}; raw={}",
            tool_call.function.name, tool_call.function.arguments
        )
    })
}

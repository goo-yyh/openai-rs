use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(feature = "tool-runner")]
use openai_rs::ToolDefinition;
use openai_rs::resources::{ChatToolChoice, ChatToolDefinition, ChatToolFunction};
use openai_rs::{ApiError, ChatCompletion, ChatCompletionToolCall, Error, ProviderKind, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// live tests 的执行层级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveTier {
    /// 最小冒烟覆盖。
    Smoke,
    /// 扩展主链路覆盖。
    Extended,
    /// 慢速或成本更高的覆盖。
    Slow,
}

impl LiveTier {
    /// 返回层级名称。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Extended => "extended",
            Self::Slow => "slow",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Smoke => 0,
            Self::Extended => 1,
            Self::Slow => 2,
        }
    }

    fn from_env() -> Self {
        match std::env::var("OPENAI_RS_LIVE_TIER")
            .unwrap_or_else(|_| "slow".into())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "smoke" => Self::Smoke,
            "extended" => Self::Extended,
            "slow" | "all" => Self::Slow,
            other => {
                eprintln!(
                    "unknown OPENAI_RS_LIVE_TIER={other}, fallback to slow for provider live tests"
                );
                Self::Slow
            }
        }
    }

    /// 判断当前环境是否允许执行指定层级。
    pub fn enabled(required: Self) -> bool {
        Self::from_env().rank() >= required.rank()
    }
}

/// 表示一条 live test 结果记录。
#[derive(Debug, Serialize)]
struct LiveCaseReport {
    timestamp_ms: u128,
    provider: String,
    case: String,
    tier: LiveTier,
    status: String,
    model: Option<String>,
    request_id: Option<String>,
    http_status: Option<u16>,
    error_kind: Option<String>,
    duration_ms: u128,
    detail: Option<String>,
}

/// 表示一条正在执行中的 live case。
#[derive(Debug, Clone)]
pub struct LiveCase {
    provider: &'static str,
    case: &'static str,
    tier: LiveTier,
    model: Option<String>,
    started: Instant,
}

impl LiveCase {
    /// 开始一条 live case；若当前 tier 不允许，会记录 skipped 并返回 `None`。
    pub fn begin(
        provider: &'static str,
        case: &'static str,
        tier: LiveTier,
        model: Option<impl Into<String>>,
    ) -> Option<Self> {
        let model = model.map(Into::into);
        if !LiveTier::enabled(tier) {
            skip_live_case(
                provider,
                case,
                tier,
                model.as_deref(),
                format!(
                    "tier {} disabled by OPENAI_RS_LIVE_TIER={}",
                    tier.as_str(),
                    LiveTier::from_env().as_str()
                ),
            );
            return None;
        }

        Some(Self {
            provider,
            case,
            tier,
            model,
            started: Instant::now(),
        })
    }

    /// 记录成功结果。
    pub fn success(self, request_id: Option<&str>, detail: impl Into<String>) {
        write_report(LiveCaseReport {
            timestamp_ms: unix_millis(),
            provider: self.provider.to_owned(),
            case: self.case.to_owned(),
            tier: self.tier,
            status: "success".into(),
            model: self.model,
            request_id: request_id.map(str::to_owned),
            http_status: None,
            error_kind: None,
            duration_ms: self.started.elapsed().as_millis(),
            detail: Some(compact_detail(detail.into())),
        });
    }

    /// 记录符合预期的 API 错误。
    pub fn expected_api_error(self, api: &ApiError, detail: impl Into<String>) {
        write_report(LiveCaseReport {
            timestamp_ms: unix_millis(),
            provider: self.provider.to_owned(),
            case: self.case.to_owned(),
            tier: self.tier,
            status: "expected_api_error".into(),
            model: self.model,
            request_id: api.request_id.clone(),
            http_status: Some(api.status),
            error_kind: Some(format!("{:?}", api.kind)),
            duration_ms: self.started.elapsed().as_millis(),
            detail: Some(compact_detail(detail.into())),
        });
    }

    /// 记录当前 case 被跳过。
    pub fn skip(self, reason: impl Into<String>) {
        write_report(LiveCaseReport {
            timestamp_ms: unix_millis(),
            provider: self.provider.to_owned(),
            case: self.case.to_owned(),
            tier: self.tier,
            status: "skipped".into(),
            model: self.model,
            request_id: None,
            http_status: None,
            error_kind: None,
            duration_ms: self.started.elapsed().as_millis(),
            detail: Some(compact_detail(reason.into())),
        });
    }
}

#[derive(Debug, Serialize, serde::Deserialize)]
struct CachedModelRecord {
    model: String,
    cached_at_secs: u64,
}

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

/// 显式记录一条 skipped live case。
pub fn skip_live_case(
    provider: &str,
    case: &str,
    tier: LiveTier,
    model: Option<&str>,
    reason: impl Into<String>,
) {
    write_report(LiveCaseReport {
        timestamp_ms: unix_millis(),
        provider: provider.to_owned(),
        case: case.to_owned(),
        tier,
        status: "skipped".into(),
        model: model.map(str::to_owned),
        request_id: None,
        http_status: None,
        error_kind: None,
        duration_ms: 0,
        detail: Some(compact_detail(reason.into())),
    });
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

/// 断言文本中不包含 Markdown 代码块包裹。
pub fn assert_no_markdown_fence(text: &str) {
    assert!(
        !text.contains("```"),
        "expected no markdown code fence, got: {text}"
    );
}

/// 断言文本中不包含常见的思维链标签。
pub fn assert_no_think_block(text: &str) {
    assert!(
        !text.contains("<think>") && !text.contains("</think>"),
        "expected no <think> block, got: {text}"
    );
}

/// 判断文本中是否包含常见的思维链标签。
pub fn contains_think_block(text: &str) -> bool {
    text.contains("<think>") || text.contains("</think>")
}

/// 断言文本中至少存在一个中文字符。
pub fn assert_contains_chinese(text: &str) {
    assert!(
        text.chars()
            .any(|ch| ('\u{4E00}'..='\u{9FFF}').contains(&ch)),
        "expected text to contain Chinese characters, got: {text}"
    );
}

/// 断言文本的句子数量不超过上限。
pub fn assert_sentence_count_at_most(text: &str, max_sentences: usize) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    let count = text
        .chars()
        .filter(|ch| matches!(ch, '。' | '！' | '？' | '.' | '!' | '?'))
        .count()
        .max(1);
    assert!(
        count <= max_sentences,
        "expected at most {max_sentences} sentence(s), got {count}: {text}"
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
            })
            .into(),
        },
    }
}

/// 构造一个简单的乘法工具定义。
pub fn multiply_numbers_tool() -> ChatToolDefinition {
    ChatToolDefinition {
        tool_type: "function".into(),
        function: ChatToolFunction {
            name: "multiply_numbers".into(),
            description: Some("Multiply two integers and return the product.".into()),
            parameters: json!({
                "type": "object",
                "properties": {
                    "a": {"type": "integer"},
                    "b": {"type": "integer"}
                },
                "required": ["a", "b"]
            })
            .into(),
        },
    }
}

/// 构造 tool-runner 版本的加法工具定义。
#[cfg(feature = "tool-runner")]
pub fn add_numbers_runner_tool() -> ToolDefinition {
    ToolDefinition::new(
        "add_numbers",
        Some("Add two integers and return a JSON object with the sum."),
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer"},
                "b": {"type": "integer"}
            },
            "required": ["a", "b"]
        }),
        |arguments: Value| async move {
            let a = arguments
                .get("a")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let b = arguments
                .get("b")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            Ok(json!({"sum": a + b}))
        },
    )
}

/// 构造 tool-runner 版本的乘法工具定义。
#[cfg(feature = "tool-runner")]
pub fn multiply_numbers_runner_tool() -> ToolDefinition {
    ToolDefinition::new(
        "multiply_numbers",
        Some("Multiply two integers and return a JSON object with the product."),
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer"},
                "b": {"type": "integer"}
            },
            "required": ["a", "b"]
        }),
        |arguments: Value| async move {
            let a = arguments
                .get("a")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let b = arguments
                .get("b")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            Ok(json!({"product": a * b}))
        },
    )
}

/// 构造强制模型选择指定函数工具的参数。
pub fn force_tool_choice(name: &str) -> ChatToolChoice {
    ChatToolChoice::function(name)
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

/// 读取指定 key 的本地缓存模型。
pub fn read_cached_model(cache_key: &str, ttl: Duration) -> Option<String> {
    let path = cache_dir().join(format!("{cache_key}.json"));
    let raw = fs::read_to_string(path).ok()?;
    let record: CachedModelRecord = serde_json::from_str(&raw).ok()?;
    let now = unix_seconds();
    if now.saturating_sub(record.cached_at_secs) > ttl.as_secs() {
        return None;
    }
    Some(record.model)
}

/// 更新指定 key 的本地缓存模型。
pub fn write_cached_model(cache_key: &str, model: &str) {
    let path = cache_dir().join(format!("{cache_key}.json"));
    let record = CachedModelRecord {
        model: model.to_owned(),
        cached_at_secs: unix_seconds(),
    };
    if let Ok(serialized) = serde_json::to_string_pretty(&record) {
        let _ = fs::write(path, serialized);
    }
}

/// 返回 ZenMux responses 模型探测缓存 TTL。
pub fn zenmux_responses_cache_ttl() -> Duration {
    let ttl = std::env::var("OPENAI_RS_ZENMUX_RESPONSES_CACHE_TTL_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(24 * 60 * 60);
    Duration::from_secs(ttl)
}

fn compact_detail(detail: String) -> String {
    const LIMIT: usize = 600;
    let detail = detail.replace('\n', " ").trim().to_owned();
    if detail.len() > LIMIT {
        format!("{}...", &detail[..LIMIT])
    } else {
        detail
    }
}

fn write_report(entry: LiveCaseReport) {
    let path = report_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string(&entry)
        && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path)
    {
        let _ = writeln!(file, "{serialized}");
    }
}

fn report_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        target_dir()
            .join("live-reports")
            .join(format!("provider_live-{}.jsonl", unix_seconds()))
    })
}

fn cache_dir() -> PathBuf {
    let dir = target_dir().join("provider-live-cache");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"))
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

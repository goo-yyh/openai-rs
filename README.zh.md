# openai-rs（中文版）

[English README](./README.md)

`openai-rs` 是一个面向 OpenAI 兼容生态的异步 Rust SDK。

> [!IMPORTANT]
> `openai-rs` 是一个社区维护的、非官方库。
>
> 它以 [openai-node](https://github.com/openai/openai-node) 作为主要参考，对其资源命名空间、能力覆盖、README 主题组织和 examples 进行了 Rust 风格重写与实现，但它不隶属于 OpenAI，也不代表 OpenAI 官方立场。

## 定位

`openai-rs` 的目标是：

- 尽可能覆盖 `openai-node` 已具备的主要功能和资源面
- 提供更符合 Rust 习惯的 builder、类型系统、错误处理和异步流接口
- 支持 OpenAI、Azure OpenAI，以及常见 OpenAI 兼容 Provider

如果你熟悉 `openai-node`，可以把它理解为：

- 能力范围尽量向 `openai-node` 对齐
- 使用方式改成 Rust 风格
- 流式接口统一落在 `futures::Stream`
- 原始 HTTP / SSE / WebSocket 细节也仍然可访问

## 版本与兼容策略

- 当前版本线：`0.1.x`
- MSRV：`1.94.1`
- Rust Edition：`2024`

当前仍处于 `0.x` 阶段：

- patch 版本不应引入有意的 breaking change
- minor 版本允许对公开 API 做收敛和重整，但会尽量附带迁移说明
- transport、provider profile、内部模块组织不属于稳定承诺的一部分

长期演进路线见 [specs/0003_improve.md](./specs/0003_improve.md)。

## 安装

默认 feature 较轻，只启用 HTTP / SSE / Multipart / Webhook 相关能力：

```toml
[dependencies]
openai-rs = "0.1"
```

如果你需要 structured output、tool runner 或 WebSocket：

```toml
[dependencies]
openai-rs = { version = "0.1", features = ["structured-output", "tool-runner", "realtime", "responses-ws"] }
```

如果你希望完全按需启用：

```toml
[dependencies]
openai-rs = { version = "0.1", default-features = false, features = ["stream", "multipart", "rustls-tls"] }
```

## Feature Flags

| Feature | 默认启用 | 说明 |
| --- | --- | --- |
| `stream` | 是 | SSE / 流式响应能力 |
| `multipart` | 是 | 文件上传与 multipart 请求支持 |
| `webhooks` | 是 | Webhook HMAC 校验 |
| `rustls-tls` | 是 | `reqwest` / WebSocket 的 rustls TLS 组合 |
| `structured-output` | 否 | `parse::<T>()`、JSON Schema 辅助与结构化输出能力 |
| `tool-runner` | 否 | 工具注册、工具执行循环和运行 trace |
| `realtime` | 否 | Realtime WebSocket 能力 |
| `responses-ws` | 否 | Responses WebSocket 能力 |

说明：

- `tool-runner` 依赖 `structured-output`
- `ws()`、`RealtimeSocket`、`ResponsesSocket` 等 WebSocket API 只会在对应 feature 开启时公开

## 快速开始

### Responses API

`Responses API` 是首选主链路，对应 `openai-node` README 里的 primary API。

```rust,ignore
use openai_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("Are semicolons optional in JavaScript?")
        .send()
        .await?;

    println!("{:?}", response.output_text());
    Ok(())
}
```

### Chat Completions API

```rust,ignore
use openai_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let completion = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_system("Talk like a pirate.")
        .message_user("Are semicolons optional in JavaScript?")
        .send()
        .await?;

    println!("{:?}", completion.choices[0].message.content);
    Ok(())
}
```

## 流式响应

`openai-rs` 对应 `openai-node` 的 SSE 能力，使用 `Stream` 暴露：

```rust,ignore
use futures_util::StreamExt;
use openai_rs::{Client, ResponseRuntimeEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let mut stream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("Say \"Sheep sleep deep\" ten times fast.")
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        if let ResponseRuntimeEvent::OutputTextDelta(event) = event? {
            print!("{}", event.text);
        }
    }

    Ok(())
}
```

对应例子：

- [examples/chat_stream.rs](./examples/chat_stream.rs)
- [examples/responses_stream.rs](./examples/responses_stream.rs)
- [examples/responses_stream_background.rs](./examples/responses_stream_background.rs)

## 文件上传

和 `openai-node` 一样，`openai-rs` 也提供统一上传 helper。

当前 `to_file()` 支持：

- `PathBuf`
- `bytes::Bytes`
- `std::io::Read`
- `tokio::io::AsyncRead`
- `reqwest::Response`
- `UploadSource`

```rust,ignore
use bytes::Bytes;
use openai_rs::{Client, to_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let file = to_file(Bytes::from_static(br#"{"hello":"world"}"#), Some("input.jsonl")).await?;

    let uploaded = client
        .files()
        .create()
        .multipart_text("purpose", "fine-tune")
        .multipart_file("file", file)
        .send()
        .await?;

    println!("{uploaded:#?}");
    Ok(())
}
```

对应例子：

- [examples/files_upload.rs](./examples/files_upload.rs)
- [examples/fine_tuning.rs](./examples/fine_tuning.rs)

## Audio

`openai-rs` 现在覆盖了：

- `audio.speech.create`
- `audio.speech` SSE 流
- `audio.transcriptions.create`
- `audio.transcriptions` SSE 流
- `audio.translations.create`
- 本地 helper：`play_audio()`、`record_audio()`

```rust,ignore
use openai_rs::{AudioPlaybackInput, Client, play_audio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let audio = client
        .audio()
        .speech()
        .create()
        .body_value(serde_json::json!({
            "model": "gpt-4o-mini-tts",
            "voice": "nova",
            "input": "Rust makes fearless concurrency practical."
        }))
        .send()
        .await?;

    play_audio(AudioPlaybackInput::bytes(audio)).await?;
    Ok(())
}
```

对应例子：

- [examples/audio_roundtrip.rs](./examples/audio_roundtrip.rs)
- [examples/text_to_speech.rs](./examples/text_to_speech.rs)
- [examples/speech_to_text.rs](./examples/speech_to_text.rs)

## Webhook 验签

和 `openai-node` README 一样，`openai-rs` 同时提供：

- 只验签：`verify_signature()`
- 验签并解析：`unwrap()`

```rust,ignore
use std::collections::BTreeMap;
use std::time::Duration;
use openai_rs::Client;

let client = Client::builder()
    .webhook_secret(std::env::var("OPENAI_WEBHOOK_SECRET")?)
    .build()?;

let raw_body = r#"{"type":"response.completed","data":{"id":"resp_123"}}"#;
let headers = BTreeMap::from([
    ("openai-signature".to_string(), "v1=dummy".to_string()),
    ("openai-timestamp".to_string(), "1735689600".to_string()),
]);

let event: serde_json::Value = client
    .webhooks()
    .unwrap(raw_body, &headers, None, Duration::from_secs(300))?;
```

对应例子：

- [examples/webhook_verification.rs](./examples/webhook_verification.rs)

## 错误处理

请求失败时会返回统一的 `openai_rs::Error`。其中 API 业务错误会落到 `Error::Api(ApiError)`：

```rust,ignore
use openai_rs::{ApiErrorKind, Error};

match client
    .chat()
    .completions()
    .create()
    .model("unknown-model")
    .message_user("hello")
    .send()
    .await
{
    Ok(response) => println!("{response:#?}"),
    Err(Error::Api(api)) => {
        println!("request_id: {:?}", api.request_id);
        println!("status: {}", api.status);
        println!("kind: {:?}", api.kind);

        if matches!(api.kind, ApiErrorKind::NotFound) {
            println!("model not found");
        }
    }
    Err(other) => return Err(other.into()),
}
```

对应例子：

- [examples/errors.rs](./examples/errors.rs)

## Request ID、原始响应和响应元信息

`openai-node` README 里强调了 request id 和 raw response 访问方式。`openai-rs` 提供两组对应能力：

- `send_with_meta()`：返回 `ApiResponse<T>`，可直接取 `meta.request_id`
- `send_raw()`：返回 `http::Response<Bytes>`

```rust,ignore
let raw = client
    .chat()
    .completions()
    .create()
    .model("gpt-5.4")
    .message_user("Say this is a test")
    .send_raw()
    .await?;

println!("{}", raw.status());

let response = client
    .chat()
    .completions()
    .create()
    .model("gpt-5.4")
    .message_user("Say this is a second test")
    .send_with_meta()
    .await?;

println!("{:?}", response.meta.request_id);
```

对应例子：

- [examples/raw_response.rs](./examples/raw_response.rs)

## 重试与超时

默认行为与 `openai-node` README 描述接近：

- 默认超时：10 分钟
- 默认重试：2 次
- 连接错误、超时、`408`、`409`、`429`、`5xx` 默认会触发重试

客户端级配置：

```rust,ignore
use std::time::Duration;

let client = openai_rs::Client::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .timeout(Duration::from_secs(20))
    .max_retries(0)
    .build()?;
```

请求级覆盖：

```rust,ignore
use std::time::Duration;

let response = client
    .responses()
    .create()
    .model("gpt-5.4")
    .input_text("How can I list all files in a directory using Python?")
    .timeout(Duration::from_secs(5))
    .max_retries(5)
    .send()
    .await?;
```

## 自动分页

列表接口返回 `CursorPage<T>`，支持：

- `has_next_page()`
- `next_page().await`
- `into_stream()`

```rust,ignore
use futures_util::StreamExt;

let first_page = client.models().list().limit(20).send().await?;
for model in &first_page.data {
    println!("{}", model.id);
}

if first_page.has_next_page() {
    let next_page = first_page.next_page().await?;
    println!("next page size = {}", next_page.data.len());
}

let mut stream = client.models().list().limit(20).send().await?.into_stream();
while let Some(model) = stream.next().await {
    println!("{}", model?.id);
}
```

对应例子：

- [examples/pagination.rs](./examples/pagination.rs)

## Logging

对应 `openai-node` README 的 logging 一节，`openai-rs` 支持：

- `OPENAI_LOG`
- `ClientBuilder::log_level(...)`
- `ClientBuilder::logger(...)`

日志级别：

- `off`
- `error`
- `warn`
- `info`
- `debug`

```rust,ignore
use std::sync::{Arc, Mutex};
use openai_rs::{LogLevel, LogRecord};

let records: Arc<Mutex<Vec<LogRecord>>> = Arc::new(Mutex::new(Vec::new()));
let sink = Arc::clone(&records);

let client = openai_rs::Client::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .log_level(LogLevel::Info)
    .logger(move |record: &LogRecord| {
        sink.lock().expect("poisoned").push(record.clone());
    })
    .build()?;
```

对应例子：

- [examples/logging.rs](./examples/logging.rs)

## Realtime 与 Responses WebSocket

`openai-rs` 当前支持：

- `client.realtime().ws()`
- `client.responses().ws()`
- `OpenAIRealtimeWebSocket`
- `OpenAIRealtimeWS`
- `OpenAIResponsesWebSocket`

Realtime 示例：

```rust,ignore
use futures_util::StreamExt;
use openai_rs::SocketStreamMessage;

let socket = client
    .realtime()
    .ws()
    .model("gpt-4o-realtime-preview")
    .connect()
    .await?;

let mut stream = socket.stream();

socket.send_json(&serde_json::json!({
    "type": "response.create",
    "response": {
        "modalities": ["text"],
        "instructions": "介绍 borrow checker"
    }
})).await?;

while let Some(event) = stream.next().await {
    match event {
        SocketStreamMessage::Message(message) => println!("{message:#?}"),
        SocketStreamMessage::Close => break,
        SocketStreamMessage::Error(error) => {
            eprintln!("{error}");
            break;
        }
        _ => {}
    }
}
```

对应例子：

- [examples/realtime_ws.rs](./examples/realtime_ws.rs)
- [examples/responses_websocket.rs](./examples/responses_websocket.rs)

更多说明见 [docs/realtime-and-streaming.md](./docs/realtime-and-streaming.md)。

## Azure OpenAI

`openai-rs` 没有单独的 `AzureOpenAI` 类，而是通过 `ClientBuilder` 上的 Azure 配置项提供同等能力：

- `azure_endpoint(...)`
- `azure_api_version(...)`
- `azure_deployment(...)`
- `azure_ad_token(...)`
- `azure_ad_token_provider(...)`

```rust,ignore
use openai_rs::Client;

let client = Client::builder()
    .azure_endpoint("https://example-resource.openai.azure.com")
    .azure_api_version("2024-02-15-preview")
    .azure_deployment("gpt-4o-prod")
    .api_key(std::env::var("AZURE_OPENAI_API_KEY")?)
    .build()?;

let response = client
    .responses()
    .create()
    .input_text("用一句话解释所有权")
    .send()
    .await?;
```

对应例子：

- [examples/azure_chat.rs](./examples/azure_chat.rs)

更多说明见 [docs/azure.md](./docs/azure.md)。

## Structured Output 与 Tool Runner

这部分是 `openai-node` 里 `helpers/zod`、`parse()`、`runTools()` 等能力在 Rust 中的对应实现。

Structured output：

```rust,ignore
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct Summary {
    title: String,
    bullets: Vec<String>,
}

let parsed = client
    .chat()
    .completions()
    .parse::<Summary>()
    .model("gpt-5.4")
    .messages(vec![
        openai_rs::ChatCompletionMessage::system("只输出 JSON。"),
        openai_rs::ChatCompletionMessage::user("返回 title 和 bullets"),
    ])
    .send()
    .await?;
```

Tool runner：

```rust,ignore
use openai_rs::ToolDefinition;
use serde_json::json;

let tool = ToolDefinition::new(
    "get_weather",
    Some("根据城市查询天气"),
    json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    }),
    |arguments: serde_json::Value| async move {
        Ok(json!({
            "city": arguments["city"].as_str().unwrap_or("unknown"),
            "weather": "sunny"
        }))
    },
);
```

对应例子：

- [examples/parsing.rs](./examples/parsing.rs)
- [examples/parsing_stream.rs](./examples/parsing_stream.rs)
- [examples/tool_runner.rs](./examples/tool_runner.rs)
- [examples/function_call.rs](./examples/function_call.rs)
- [examples/function_call_stream.rs](./examples/function_call_stream.rs)
- [examples/function_call_stream_raw.rs](./examples/function_call_stream_raw.rs)
- [examples/responses_streaming_tools.rs](./examples/responses_streaming_tools.rs)
- [examples/responses_structured_outputs.rs](./examples/responses_structured_outputs.rs)
- [examples/responses_structured_outputs_tools.rs](./examples/responses_structured_outputs_tools.rs)

更多说明见 [docs/structured-output-and-tools.md](./docs/structured-output-and-tools.md)。

## Examples

### 示例运行

```bash
cargo run --example openai_responses
cargo run --example chat_params_types
cargo run --example chat_stream
cargo run --example raw_response
cargo run --example pagination
cargo run --example stream_to_client_sse
cargo run --example stream_to_client_ndjson
cargo run --example tool_runner --features tool-runner
cargo run --example parsing --features structured-output
cargo run --example ui_generation --features structured-output
cargo run --example parsing_tools --features structured-output
cargo run --example realtime_ws --features realtime
cargo run --example azure_realtime_ws --features realtime
cargo run --example responses_websocket --features responses-ws
```

完整索引见 [docs/examples.md](./docs/examples.md)。

### `openai-node/examples` 覆盖映射

下面这张表把 `openai-node/examples` 的主题映射到 `openai-rs/examples` 的对应样例。Rust 端不会机械复制 Node 的每个运行时包装，但对应能力都能在这些例子中找到。

| `openai-node` 示例 | `openai-rs` 对应示例 |
| --- | --- |
| `demo.ts`, `types.ts` | [examples/openai_chat.rs](./examples/openai_chat.rs), [examples/openai_responses.rs](./examples/openai_responses.rs) |
| `chat-params-types.ts` | [examples/chat_params_types.rs](./examples/chat_params_types.rs) |
| `stream.ts` | [examples/chat_stream.rs](./examples/chat_stream.rs) |
| `logprobs.ts` | [examples/logprobs.rs](./examples/logprobs.rs) |
| `function-call.ts`, `function-call-diy.ts` | [examples/function_call.rs](./examples/function_call.rs) |
| `function-call-stream.ts` | [examples/function_call_stream.rs](./examples/function_call_stream.rs) |
| `function-call-stream-raw.ts`, `tool-calls-stream.ts` | [examples/function_call_stream_raw.rs](./examples/function_call_stream_raw.rs), [examples/function_call_stream.rs](./examples/function_call_stream.rs) |
| `tool-call-helpers.ts`, `tool-call-helpers-zod.ts`, `parsing-run-tools.ts` | [examples/tool_runner.rs](./examples/tool_runner.rs) |
| `parsing.ts` | [examples/parsing.rs](./examples/parsing.rs) |
| `parsing-tools.ts` | [examples/parsing_tools.rs](./examples/parsing_tools.rs) |
| `ui-generation.ts` | [examples/ui_generation.rs](./examples/ui_generation.rs) |
| `parsing-stream.ts` | [examples/parsing_stream.rs](./examples/parsing_stream.rs) |
| `parsing-tools-stream.ts` | [examples/parsing_tools_stream.rs](./examples/parsing_tools_stream.rs) |
| `assistants.ts` | [examples/assistants_poll.rs](./examples/assistants_poll.rs) |
| `assistant-stream.ts` | [examples/assistants_stream.rs](./examples/assistants_stream.rs) |
| `assistant-stream-raw.ts` | [examples/assistants_stream_raw.rs](./examples/assistants_stream_raw.rs) |
| `audio.ts` | [examples/audio_roundtrip.rs](./examples/audio_roundtrip.rs) |
| `speech-to-text.ts` | [examples/speech_to_text.rs](./examples/speech_to_text.rs) |
| `text-to-speech.ts` | [examples/text_to_speech.rs](./examples/text_to_speech.rs) |
| `image-stream.ts` | [examples/image_stream.rs](./examples/image_stream.rs) |
| `errors.ts` | [examples/errors.rs](./examples/errors.rs) |
| `raw-response.ts` | [examples/raw_response.rs](./examples/raw_response.rs) |
| `fine-tuning.ts` | [examples/fine_tuning.rs](./examples/fine_tuning.rs) |
| `azure/chat.ts` | [examples/azure_chat.rs](./examples/azure_chat.rs) |
| `azure/realtime.ts` | [examples/azure_realtime_ws.rs](./examples/azure_realtime_ws.rs) |
| `realtime/websocket.ts`, `realtime/ws.ts` | [examples/realtime_ws.rs](./examples/realtime_ws.rs) |
| `responses/stream.ts` | [examples/responses_stream.rs](./examples/responses_stream.rs) |
| `responses/stream_background.ts` | [examples/responses_stream_background.rs](./examples/responses_stream_background.rs) |
| `responses/streaming-tools.ts` | [examples/responses_streaming_tools.rs](./examples/responses_streaming_tools.rs) |
| `responses/structured-outputs.ts` | [examples/responses_structured_outputs.rs](./examples/responses_structured_outputs.rs) |
| `responses/structured-outputs-tools.ts` | [examples/responses_structured_outputs_tools.rs](./examples/responses_structured_outputs_tools.rs) |
| `responses/websocket.ts` | [examples/responses_websocket.rs](./examples/responses_websocket.rs) |
| `stream-to-client-browser.ts`, `stream-to-client-express.ts`, `stream-to-client-next.ts` | [examples/stream_to_client_sse.rs](./examples/stream_to_client_sse.rs), [examples/stream_to_client_ndjson.rs](./examples/stream_to_client_ndjson.rs) |
| `stream-to-client-raw.ts` | [examples/stream_to_client_raw.rs](./examples/stream_to_client_raw.rs) |

补充说明：

- Node/browser/framework 绑定较强的示例，在 Rust 侧统一用框架无关的 raw forwarding 方式展示
- Node 里依赖 `zod` 的示例，在 Rust 侧改用 `serde` + `schemars` + `parse::<T>()`
- Node 里依赖 emitter 的示例，在 Rust 侧改用 `Stream` 和 runtime event

## Provider 支持矩阵

| Provider | 支持级别 | 说明 |
| --- | --- | --- |
| OpenAI | 一等支持 | 默认行为与测试覆盖优先围绕 OpenAI 语义设计 |
| Azure OpenAI | 一等支持 | 支持 endpoint、deployment、`api-version`、`api-key` 与 Azure AD token |
| Zhipu | 兼容支持 | 通过兼容层适配，行为以 provider 实际实现为准 |
| MiniMax | 兼容支持 | 通过兼容层适配，行为以 provider 实际实现为准 |
| ZenMux | 兼容支持 | 通过兼容层适配，行为以 provider 实际实现为准 |
| Custom Provider | 扩展支持 | SDK 提供稳定入口，具体兼容行为由接入方自行保证 |

更细的说明见 [docs/provider-capability-matrix.md](./docs/provider-capability-matrix.md)。

## 专题文档

- [Azure OpenAI 接入](./docs/azure.md)
- [API 参考总览](./docs/api-reference.md)
- [Examples 索引](./docs/examples.md)
- [FAQ](./docs/faq.md)
- [OpenAPI contract 维护](./docs/openapi-contract.md)
- [流式与 Realtime](./docs/realtime-and-streaming.md)
- [Structured Output 与 Tool Runner](./docs/structured-output-and-tools.md)
- [迁移说明](./docs/migration.md)
- [可观测性说明](./docs/observability.md)
- [Provider 能力矩阵](./docs/provider-capability-matrix.md)
- [public API 维护说明](./docs/public-api.md)
- [发布检查清单](./docs/release-checklist.md)
- [ADR: codegen 策略](./docs/adr/0001_codegen_strategy.md)

## 校验与开发

常用检查命令：

```bash
cargo build
cargo test
cargo check --no-default-features
cargo check --no-default-features --features structured-output,tool-runner
cargo check --no-default-features --features realtime,responses-ws
cargo check --examples --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
bash ./scripts/check-public-api.sh
```

补充说明：

- `tests/provider_live/` 下的 live smoke tests 默认 `#[ignore]`
- 若缺少对应环境变量，这些 live tests 会自动跳过

## FAQ

简版结论：

- `openai-rs` 是社区维护 SDK，不是官方 SDK
- 默认 feature 故意保持精简，`realtime`、`responses-ws`、`structured-output` 都按需开启
- 做旧代码迁移时优先用 `chat().completions()`，希望靠近新接口时优先评估 `responses()`
- live provider tests 默认手动执行，因为它们依赖真实凭据并可能产生成本

更完整说明见 [docs/faq.md](./docs/faq.md)。

## 项目状态

当前仓库已经具备可发布的 SDK 基础能力，后续优化重点放在：

- 公开 API 面进一步收缩
- 长尾资源继续强类型化
- 文档和 examples 继续增强
- feature matrix 与公共 API 稳定性治理

如果你的目标是：

- 找一个和 `openai-node` 能力覆盖尽量接近的 Rust SDK
- 接受它是社区维护、非官方实现
- 更偏好 Rust builder / type / async stream 的接口风格

那么 `openai-rs` 当前已经适合作为主 SDK 使用。

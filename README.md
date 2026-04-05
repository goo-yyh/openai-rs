# openai-rs

`openai-rs` 是一个面向 OpenAI 兼容生态的异步 Rust SDK。

当前实现目标是：

- 对齐 `openai-node` 的主要公开能力
- 提供 Rust 风格的 builder、强类型和异步体验
- 兼容 OpenAI、Azure OpenAI 与常见 OpenAI 兼容 Provider

## 版本与兼容策略

- 当前版本线：`0.1.x`
- MSRV：`1.94.1`
- Rust Edition：`2024`

当前仍处于 `0.x` 阶段，版本策略如下：

- patch 版本不应引入有意的 breaking change
- minor 版本允许对公开 API 做收敛和重整，但会尽量附带迁移说明
- 内部实现细节不是稳定承诺的一部分，例如 transport 内部拼装逻辑、provider 内部 profile 细节、非公开模块结构

长期优化路线见 [specs/0003_improve.md](./specs/0003_improve.md)。

## Provider 支持矩阵

| Provider | 支持级别 | 说明 |
| --- | --- | --- |
| OpenAI | 一等支持 | 默认行为与测试覆盖优先围绕 OpenAI 语义设计 |
| Azure OpenAI | 一等支持 | 支持 endpoint、deployment、`api-version`、`api-key` 与 Azure AD token |
| Zhipu | 兼容支持 | 通过兼容层适配，行为以 provider 实际实现为准 |
| MiniMax | 兼容支持 | 通过兼容层适配，行为以 provider 实际实现为准 |
| ZenMux | 兼容支持 | 通过兼容层适配，行为以 provider 实际实现为准 |
| Custom Provider | 扩展支持 | SDK 提供稳定入口，具体兼容行为由接入方自行保证 |

## 安装

默认 feature 较轻，只启用 HTTP / SSE / Multipart / Webhook 相关能力：

```toml
[dependencies]
openai-rs = "0.1"
```

如果你需要结构化输出、工具调用或 WebSocket，再按需开启：

```toml
[dependencies]
openai-rs = { version = "0.1", features = ["structured-output", "tool-runner", "realtime", "responses-ws"] }
```

如果你希望完全按需选择能力：

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
| `tool-runner` | 否 | tool runner、工具注册与自动工具调用循环 |
| `realtime` | 否 | Realtime WebSocket 能力 |
| `responses-ws` | 否 | Responses WebSocket 能力 |

说明：

- `tool-runner` 依赖 `structured-output`
- `ws()`、`RealtimeSocket`、`ResponsesSocket` 等 WebSocket API 只会在对应 feature 开启时公开

## 专题文档

- [Azure OpenAI 接入](./docs/azure.md)
- [流式与 Realtime](./docs/realtime-and-streaming.md)
- [Structured Output 与 Tool Runner](./docs/structured-output-and-tools.md)
- [迁移说明](./docs/migration.md)
- [可观测性说明](./docs/observability.md)
- [发布检查清单](./docs/release-checklist.md)
- [ADR: codegen 策略](./docs/adr/0001_codegen_strategy.md)

## 快速开始

### OpenAI 聊天补全

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
        .message_system("你是一个 Rust 助手")
        .message_user("解释 Tokio 的运行时模型")
        .send()
        .await?;

    println!("{completion:#?}");
    Ok(())
}
```

### Azure OpenAI

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

如需 Azure AD / Entra Bearer Token：

```rust,ignore
use openai_rs::Client;
use secrecy::SecretString;

let client = Client::builder()
    .azure_endpoint("https://example-resource.openai.azure.com")
    .azure_api_version("2024-02-15-preview")
    .azure_ad_token_provider(|| async {
        Ok(SecretString::new("azure-ad-token".into()))
    })
    .build()?;
```

### Responses API

```rust,ignore
let response = client
    .responses()
    .create()
    .model("gpt-5.4")
    .input_text("给我一段简短的 Rust 所有权解释")
    .send()
    .await?;

println!("{:?}", response.output_text());
```

### Structured Output

需要 `structured-output` feature。

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
    .message_user("返回 JSON：title + bullets")
    .send()
    .await?;
```

### Tool Runner

需要 `tool-runner` feature。

```rust,ignore
use openai_rs::ToolDefinition;
use serde_json::json;

let weather_tool = ToolDefinition::new(
    "get_weather",
    Some("根据城市查询天气"),
    json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    }),
    |arguments| async move {
        let city = arguments["city"].as_str().unwrap_or("unknown");
        Ok(json!({ "city": city, "weather": "sunny" }))
    },
);

let final_response = client
    .chat()
    .completions()
    .run_tools()
    .model("gpt-5.4")
    .message_user("查询上海天气")
    .register_tool(weather_tool)
    .send()
    .await?;
```

### Realtime WebSocket

需要 `realtime` feature。

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
if let Some(SocketStreamMessage::Open) = stream.next().await {
    socket
        .send_json(&serde_json::json!({
            "type": "response.create",
            "response": {
                "modalities": ["text"],
                "instructions": "介绍 borrow checker"
            }
        }))
        .await?;
}
```

Azure Realtime 可以直接复用 client 上配置好的 deployment：

```rust,ignore
let socket = client.realtime().ws().connect().await?;
```

### Responses WebSocket

需要 `responses-ws` feature。

```rust,ignore
use futures_util::StreamExt;
use openai_rs::SocketStreamMessage;

let socket = client.responses().ws().connect().await?;
let mut stream = socket.stream();

socket
    .send_json(&serde_json::json!({
        "type": "response.create",
        "response": {
            "model": "gpt-5.4",
            "input": "hello"
        }
    }))
    .await?;

while let Some(event) = stream.next().await {
    match event {
        SocketStreamMessage::Message(message) => println!("{message:#?}"),
        SocketStreamMessage::Error(error) => {
            eprintln!("{error}");
            break;
        }
        SocketStreamMessage::Close => break,
        _ => {}
    }
}
```

## 资源结构

`Client` 当前暴露与 `openai-node` 对齐的主要顶层命名空间，包括但不限于：

- `chat`
- `responses`
- `models`
- `files`
- `uploads`
- `images`
- `audio`
- `moderations`
- `vector_stores`
- `fine_tuning`
- `batches`
- `webhooks`
- `realtime`
- `beta`

高频路径优先提供强类型实现，长尾路径统一收敛到通用 request builder。

当前已经优先把以下长尾资源提升为强类型返回值：

- `beta.assistants`
- `beta.threads`
- `beta.threads.messages`
- `beta.threads.runs`
- `beta.threads.runs.steps`
- `vector_stores`
- `vector_stores.files`
- `vector_stores.file_batches`

## Examples

仓库内置了可编译示例：

- [examples/openai_chat.rs](./examples/openai_chat.rs)
- [examples/openai_responses.rs](./examples/openai_responses.rs)
- [examples/azure_chat.rs](./examples/azure_chat.rs)
- [examples/realtime_ws.rs](./examples/realtime_ws.rs)
- [examples/files_upload.rs](./examples/files_upload.rs)

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

## 项目状态

当前仓库已经具备可发布的 SDK 基础能力，后续优化重点放在：

- 公开 API 面进一步收缩
- `resources` 目录拆分
- 长尾资源强类型化
- feature matrix 与公共 API 稳定性治理

对应规划见 [specs/0003_improve.md](./specs/0003_improve.md)。

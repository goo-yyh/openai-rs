# openai-rs

`openai-rs` 是一个面向 OpenAI 兼容生态的异步 Rust SDK，实现目标是对齐 `openai-node` 的公开功能面，同时保留 Rust 侧更直接的强类型与 builder 体验。

当前版本已经覆盖：

- OpenAI、Azure OpenAI、智谱、MiniMax、ZenMux 与自定义 Provider
- 标准 HTTP 请求、SSE 流、Realtime WebSocket、Responses WebSocket
- 超时、重试、默认 header/query 合并、统一错误映射
- Multipart 上传、Webhook HMAC 校验、Structured Output、Tool Runner
- `openai-node` 对应的顶层资源命名空间与主要子资源
- 高频路径强类型实现，长尾路径统一收敛到通用 request builder

默认 feature 已启用：

- `stream`
- `multipart`
- `webhooks`
- `structured-output`
- `tool-runner`
- `realtime`
- `responses-ws`
- `rustls-tls`

## 工具链

- Rust stable `1.94.1`
- Edition `2024`

## 安装

```toml
[dependencies]
openai-rs = { path = "./openai-rs" }
```

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
    .model("ignored-when-deployment-is-injected")
    .input_text("用一句话解释所有权")
    .send()
    .await?;
```

若使用 Microsoft Entra / Azure AD Token，可直接切到 Bearer 模式：

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

Azure Realtime 可以直接复用 builder 中配置的 deployment：

```rust,ignore
let socket = client.realtime().ws().connect().await?;
```

### Responses WebSocket

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
        SocketStreamMessage::Message(message) => {
            println!("{message:#?}");
        }
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

`Client` 已暴露与 `openai-node` 对齐的顶层命名空间：

- `completions`
- `chat`
- `embeddings`
- `files`
- `images`
- `audio`
- `moderations`
- `models`
- `fine_tuning`
- `graders`
- `vector_stores`
- `webhooks`
- `batches`
- `uploads`
- `responses`
- `realtime`
- `conversations`
- `evals`
- `containers`
- `skills`
- `videos`
- `beta`

实现策略分两层：

- 高频接口使用强类型 builder，例如 `chat.completions`、`responses`、`models`、`files`、`uploads`
- 长尾接口统一使用 `JsonRequestBuilder<T>`、`BytesRequestBuilder`、`ListRequestBuilder<T>`

这样可以保持功能面完整，同时避免把所有资源都膨胀成极重的样板类型层。

## 关键能力

- `Client::with_options` 支持克隆后局部覆写配置
- `ClientBuilder::api_key_async_provider` 支持异步凭证回调
- Azure 支持 `endpoint`、`api-version`、`deployment`、`api-key`、Bearer Token / Azure AD Token
- `ChatCompletionStream` 与 `ResponseStream` 会做常见增量聚合
- `RealtimeSocket` 与 `ResponsesSocket` 提供统一的生命周期事件流
- `WebhookVerifier` 支持签名校验与事件解包

## 测试

主要测试目录：

```text
tests/
├── contract/
├── provider_live/
└── websocket.rs
```

当前默认测试覆盖：

- Provider 默认配置与严格模式校验
- Azure 路径改写、deployment 注入与 Bearer / `api-key` 鉴权
- Client 默认 header/query 合并
- 聊天补全、Responses、分页、Structured Output、Tool Runner 关键链路
- SSE 行解码、Multipart 展开、Webhook 验签
- Realtime WebSocket 与 Responses WebSocket 握手、鉴权与事件流

验证命令：

```bash
cargo build
cargo test
cargo +nightly fmt
cargo clippy --all-targets --all-features -- -D warnings
```

真实 provider 的 smoke tests 位于 `tests/provider_live/`，默认使用 `#[ignore]`。

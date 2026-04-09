# openai-rs

[中文版 README](./README.zh.md)

`openai-rs` is an async Rust SDK for the OpenAI-compatible ecosystem.

> [!IMPORTANT]
> `openai-rs` is a community-maintained, unofficial library.
>
> It is a Rust rewrite heavily informed by [openai-node](https://github.com/openai/openai-node): its resource layout, capability coverage, README structure, and example topics were all reviewed against `openai-node`, then adapted into Rust-native builders, types, and async streams. It is not affiliated with OpenAI and does not represent an official OpenAI SDK.

## Positioning

The project aims to:

- cover the major capability surface already available in `openai-node`
- provide Rust-native builders, typed models, async streams, and error handling
- support OpenAI, Azure OpenAI, and common OpenAI-compatible providers

If you already know `openai-node`, the rough mental model is:

- the capability surface tries to stay close to `openai-node`
- the public API is intentionally Rust-flavored instead of mirroring TypeScript shapes
- streaming uses `futures::Stream`
- raw HTTP, SSE, and WebSocket primitives are still accessible when needed

## Versioning and Compatibility

- Current line: `0.1.x`
- MSRV: `1.94.1`
- Rust edition: `2024`

The crate is still in `0.x`:

- patch releases should not intentionally introduce breaking changes
- minor releases may still reshape parts of the public API, with migration notes when practical
- internal transport logic, provider profile internals, and private module structure are not part of the stability contract

Longer-term planning lives in [specs/0003_improve.md](./specs/0003_improve.md).

## Installation

The default feature set is intentionally lighter and focuses on HTTP, SSE, multipart, and webhooks:

```toml
[dependencies]
openai-rs = "0.1"
```

If you also need structured output, tool runners, or WebSocket support:

```toml
[dependencies]
openai-rs = { version = "0.1", features = ["structured-output", "tool-runner", "realtime", "responses-ws"] }
```

If you want full control over features:

```toml
[dependencies]
openai-rs = { version = "0.1", default-features = false, features = ["stream", "multipart", "rustls-tls"] }
```

## Feature Flags

| Feature | Enabled by default | Purpose |
| --- | --- | --- |
| `stream` | Yes | SSE and streaming response support |
| `multipart` | Yes | File uploads and multipart requests |
| `webhooks` | Yes | Webhook HMAC verification |
| `rustls-tls` | Yes | rustls-based TLS for `reqwest` and WebSockets |
| `structured-output` | No | `parse::<T>()`, JSON Schema helpers, structured outputs |
| `tool-runner` | No | tool registration, tool execution loops, runner traces |
| `realtime` | No | Realtime WebSocket support |
| `responses-ws` | No | Responses WebSocket support |

Notes:

- `tool-runner` depends on `structured-output`
- WebSocket APIs such as `ws()`, `RealtimeSocket`, and `ResponsesSocket` are only exported when their features are enabled

## Quick Start

### Responses API

The `Responses API` is the primary path, matching the role it plays in the `openai-node` README.

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

## Streaming Responses

Like `openai-node`, `openai-rs` supports Server-Sent Events. The Rust-facing shape uses async streams instead of emitters.

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

Related examples:

- [examples/chat_stream.rs](./examples/chat_stream.rs)
- [examples/responses_stream.rs](./examples/responses_stream.rs)
- [examples/responses_stream_background.rs](./examples/responses_stream_background.rs)

## File Uploads

Just like `openai-node`, `openai-rs` exposes a unified upload helper.

`to_file()` currently accepts:

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

Related examples:

- [examples/files_upload.rs](./examples/files_upload.rs)
- [examples/fine_tuning.rs](./examples/fine_tuning.rs)

## Audio

`openai-rs` now covers:

- `audio.speech.create`
- SSE streaming for `audio.speech`
- `audio.transcriptions.create`
- SSE streaming for `audio.transcriptions`
- `audio.translations.create`
- local helper utilities: `play_audio()` and `record_audio()`

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
        .model("gpt-4o-mini-tts")
        .voice("nova")
        .input("Rust makes fearless concurrency practical.")
        .send()
        .await?;

    play_audio(AudioPlaybackInput::bytes(audio)).await?;
    Ok(())
}
```

Related examples:

- [examples/audio_roundtrip.rs](./examples/audio_roundtrip.rs)
- [examples/text_to_speech.rs](./examples/text_to_speech.rs)
- [examples/speech_to_text.rs](./examples/speech_to_text.rs)

### Typed Long-tail Resources

Phase 3 promotes the main long-tail namespaces away from raw `Value` builders:

- `images`
- `audio`
- `fine_tuning`
- `batches`
- `conversations`
- `evals`
- `containers`
- `skills`
- `videos`

For the high-frequency paths, you can now use typed responses plus either dedicated builder methods or typed request structs with `json_body(...)`.

```rust,ignore
use openai_rs::{Client, ConversationCreateParams};

let client = Client::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .build()?;

let conversation = client
    .conversations()
    .create()
    .json_body(&ConversationCreateParams {
        name: Some("demo".into()),
        ..ConversationCreateParams::default()
    })?
    .send()
    .await?;

println!("{}", conversation.id);
```

## Webhook Verification

As in `openai-node`, `openai-rs` provides both:

- signature-only verification via `verify_signature()`
- verify-and-parse via `unwrap()`

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

Related example:

- [examples/webhook_verification.rs](./examples/webhook_verification.rs)

## Error Handling

Failures are exposed through the unified `openai_rs::Error` type. API-level failures are represented as `Error::Api(ApiError)`.

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

Related example:

- [examples/errors.rs](./examples/errors.rs)

## Request IDs, Raw Responses, and Response Metadata

`openai-node` emphasizes request ids and raw response access. `openai-rs` exposes the same debugging surface through two methods:

- `send_with_meta()` returns `ApiResponse<T>` so you can read `meta.request_id`
- `send_raw()` returns `http::Response<Bytes>`

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

Related example:

- [examples/raw_response.rs](./examples/raw_response.rs)

## Retries and Timeouts

The default behavior is intentionally close to the behavior documented in `openai-node`:

- default timeout: 10 minutes
- default retries: 2
- connection errors, timeouts, `408`, `409`, `429`, and `5xx` responses are retried by default

Client-wide configuration:

```rust,ignore
use std::time::Duration;

let client = openai_rs::Client::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .timeout(Duration::from_secs(20))
    .max_retries(0)
    .build()?;
```

Per-request overrides:

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

## Auto Pagination

List APIs return `CursorPage<T>`, which supports:

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

Related example:

- [examples/pagination.rs](./examples/pagination.rs)

## Logging

Matching the `openai-node` README topic, `openai-rs` supports:

- `OPENAI_LOG`
- `ClientBuilder::log_level(...)`
- `ClientBuilder::logger(...)`

Available levels:

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

Related example:

- [examples/logging.rs](./examples/logging.rs)

## Realtime and Responses WebSocket

`openai-rs` currently supports:

- `client.realtime().ws()`
- `client.responses().ws()`
- `OpenAIRealtimeWebSocket`
- `OpenAIRealtimeWS`
- `OpenAIResponsesWebSocket`

Realtime example:

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
        "instructions": "Explain the borrow checker"
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

Related examples:

- [examples/realtime_ws.rs](./examples/realtime_ws.rs)
- [examples/responses_websocket.rs](./examples/responses_websocket.rs)

More background: [docs/realtime-and-streaming.md](./docs/realtime-and-streaming.md).

## Azure OpenAI

`openai-rs` does not expose a separate `AzureOpenAI` class. Instead, the same capability is configured through `ClientBuilder`:

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
    .input_text("Explain ownership in one sentence")
    .send()
    .await?;
```

Related example:

- [examples/azure_chat.rs](./examples/azure_chat.rs)

More background: [docs/azure.md](./docs/azure.md).

## Structured Output and Tool Runners

This is the Rust-side answer to the `helpers/zod`, `parse()`, and `runTools()` story in `openai-node`.

Structured output:

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
        openai_rs::ChatCompletionMessage::system("Only output JSON."),
        openai_rs::ChatCompletionMessage::user("Return title and bullets"),
    ])
    .send()
    .await?;
```

Tool runner:

```rust,ignore
use openai_rs::ToolDefinition;
use serde_json::json;

let tool = ToolDefinition::new(
    "get_weather",
    Some("Fetch weather by city"),
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

Related examples:

- [examples/parsing.rs](./examples/parsing.rs)
- [examples/parsing_stream.rs](./examples/parsing_stream.rs)
- [examples/tool_runner.rs](./examples/tool_runner.rs)
- [examples/function_call.rs](./examples/function_call.rs)
- [examples/function_call_stream.rs](./examples/function_call_stream.rs)
- [examples/function_call_stream_raw.rs](./examples/function_call_stream_raw.rs)
- [examples/responses_streaming_tools.rs](./examples/responses_streaming_tools.rs)
- [examples/responses_structured_outputs.rs](./examples/responses_structured_outputs.rs)
- [examples/responses_structured_outputs_tools.rs](./examples/responses_structured_outputs_tools.rs)

More background: [docs/structured-output-and-tools.md](./docs/structured-output-and-tools.md).

## Examples

### Running Examples

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

Full index: [docs/examples.md](./docs/examples.md)

### Coverage Mapping Against `openai-node/examples`

The table below maps `openai-node/examples` topics to their Rust equivalents. The Rust side does not mechanically duplicate every Node runtime wrapper, but the underlying capabilities are represented here.

| `openai-node` example | `openai-rs` example(s) |
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

Notes:

- examples that are strongly tied to Node, browsers, or web frameworks are represented on the Rust side using framework-neutral raw forwarding patterns
- Node examples that rely on `zod` map to `serde` + `schemars` + `parse::<T>()` in Rust
- emitter-based examples map to `Stream` plus runtime event enums

## Provider Support Matrix

| Provider | Support level | Notes |
| --- | --- | --- |
| OpenAI | First-class | The main compatibility target and the primary focus for behavior and tests |
| Azure OpenAI | First-class | Supports endpoint, deployment, `api-version`, `api-key`, and Azure AD tokens |
| Zhipu | Compatibility | Routed through the compatibility layer; real behavior depends on the provider |
| MiniMax | Compatibility | Routed through the compatibility layer; real behavior depends on the provider |
| ZenMux | Compatibility | Routed through the compatibility layer; real behavior depends on the provider |
| Custom providers | Extensible | The SDK exposes stable integration points; final compatibility depends on the integrator |

More detail: [docs/provider-capability-matrix.md](./docs/provider-capability-matrix.md)

## Topic Guides

- [Azure integration](./docs/azure.md)
- [API reference](./docs/api-reference.md)
- [Examples index](./docs/examples.md)
- [FAQ](./docs/faq.md)
- [OpenAPI contract maintenance](./docs/openapi-contract.md)
- [Streaming and Realtime](./docs/realtime-and-streaming.md)
- [Structured output and tools](./docs/structured-output-and-tools.md)
- [Migration notes](./docs/migration.md)
- [Observability](./docs/observability.md)
- [Provider capability matrix](./docs/provider-capability-matrix.md)
- [Public API maintenance](./docs/public-api.md)
- [Release checklist](./docs/release-checklist.md)
- [ADR: codegen strategy](./docs/adr/0001_codegen_strategy.md)

## Development Checks

Common verification commands:

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

Additional notes:

- live smoke tests under `tests/provider_live/` are `#[ignore]` by default
- when required environment variables are missing, those live tests auto-skip

## FAQ

The short version:

- `openai-rs` is a community SDK, not an official OpenAI SDK
- default features are intentionally small; realtime / responses-ws / structured-output stay opt-in
- use `chat().completions()` for legacy-compatible migrations, and `responses()` when you want the newer API surface
- live provider tests are manual by design because they consume real credentials and may incur cost

More detail: [docs/faq.md](./docs/faq.md)

## Project Status

The crate already has a publishable SDK baseline. The next round of work is focused more on refinement than on basic capability gaps:

- further tightening the stable public API surface
- continuing to strongly type long-tail resources
- expanding docs and examples
- hardening feature-matrix and public-API stability checks

If your goal is:

- a Rust SDK that tracks the functional surface of `openai-node` closely
- a community-maintained, unofficial implementation
- Rust-native builders, types, and async streams instead of TypeScript emitter semantics

then `openai-rs` is already a strong primary SDK candidate.

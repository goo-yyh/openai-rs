# openai-core

`openai-core` is an async Rust SDK for the OpenAI-compatible ecosystem.

It is a community-maintained, unofficial library. The crate tracks the main capability surface exposed by `openai-node`, then adapts it into Rust-native builders, typed models, async streams, and provider abstractions.

## Start Here

- [`Client`] is the main entry point for API calls.
- [`ClientBuilder`] configures auth, provider selection, base URL overrides, retries, and timeouts.
- [`Provider`] and [`ClientOptions`] cover OpenAI, Azure OpenAI, and compatible providers.
- [`resources`] exposes typed request and response models for chat, responses, files, fine-tuning, vector stores, realtime, and long-tail namespaces.
- [`stream`] exposes SSE and runtime stream types such as [`ResponseStream`], [`ChatCompletionStream`], and [`AssistantStream`].
- [`files`] and [`to_file`] provide unified upload helpers for multipart requests.
- [`webhooks`] contains signature verification helpers such as [`WebhookVerifier`] and [`WebhookEvent`].
- [`pagination`] contains page wrappers and async pagination helpers.
- [`JsonPayload`] is the forward-compatible escape hatch when a provider returns raw JSON that does not yet have a dedicated typed wrapper.

## Installation

```toml
[dependencies]
openai-core = "0.1"
```

Enable extra features only when you need them:

```toml
[dependencies]
openai-core = { version = "0.1", features = ["structured-output", "tool-runner", "realtime", "responses-ws"] }
```

## Feature Flags

| Feature | Default | Purpose |
| --- | --- | --- |
| `stream` | Yes | SSE and streaming response support |
| `multipart` | Yes | File uploads and multipart requests |
| `webhooks` | Yes | Webhook HMAC verification |
| `rustls-tls` | Yes | rustls-based TLS for `reqwest` and WebSockets |
| `structured-output` | No | `parse::<T>()`, JSON Schema helpers, typed structured outputs |
| `tool-runner` | No | Tool registration, tool execution loops, and runner traces |
| `realtime` | No | Realtime WebSocket support |
| `responses-ws` | No | Responses WebSocket support |

`docs.rs` for this crate is built with all features enabled, so feature-gated items are visible in the generated API pages and marked with `doc(cfg(...))`.

## Quick Start

```rust,ignore
use openai_core::Client;

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

## Common Workflows

### Responses API

Use [`Client`] with `.responses()` for the primary documented API path. Key types include [`Response`], [`ResponseOutputItem`], [`ResponseOutputContentPart`], and [`ResponseUsage`].

### Chat Completions API

Use `.chat().completions()` when you want the compatibility path that maps more directly to classic chat completion workflows. Key types include [`ChatCompletion`], [`ChatCompletionChunk`], and [`ChatCompletionMessage`].

### Streaming

For SSE and event-driven flows, start from `.responses().stream()` or `.chat().completions().stream()`. The stream runtime is documented under [`stream`], with high-level types such as [`ResponseStream`], [`ChatCompletionStream`], [`AssistantStream`], and [`RawSseStream`].

### Uploads and Multipart

Use [`to_file`], [`UploadSource`], and [`FileLike`] to normalize byte buffers, local files, readers, and HTTP responses into multipart uploads.

### Webhooks

Use [`WebhookVerifier`] to validate signatures and [`WebhookEvent`] to deserialize incoming webhook payloads.

### Pagination

Use [`Page`], [`CursorPage`], and [`PageStream`] when iterating across paginated list endpoints.

### Realtime and Responses WebSocket

Realtime and WebSocket APIs are available behind the `realtime` and `responses-ws` features. The generated API docs include the exported socket and event types when those features are enabled.

## API Map

- Core client types: [`Client`], [`ClientBuilder`], [`ClientOptions`], [`RequestOptions`]
- Provider abstractions: [`Provider`], [`ProviderKind`], [`ProviderProfile`]
- Files and uploads: [`files`], [`FileObject`], [`UploadObject`]
- Models and resources: [`resources`], [`Model`], [`DeleteResponse`]
- Streaming: [`stream`], [`SseStream`], [`ResponseEventStream`], [`ChatCompletionEventStream`]
- Errors and metadata: [`Error`], [`ApiError`], [`ResponseMeta`], [`ApiResponse`]

## Guides and Examples

- [Repository README][repo-readme]
- [Chinese README][repo-readme-zh]
- [Examples index][repo-examples]
- [API reference overview][repo-api-reference]
- [Azure guide][repo-azure]
- [Streaming and Realtime guide][repo-streaming]
- [Structured output and tools guide][repo-structured]
- [Provider capability matrix][repo-provider-matrix]
- [FAQ][repo-faq]

[repo-readme]: https://github.com/goo-yyh/openai-rs/blob/main/README.md
[repo-readme-zh]: https://github.com/goo-yyh/openai-rs/blob/main/README.zh.md
[repo-examples]: https://github.com/goo-yyh/openai-rs/blob/main/docs/examples.md
[repo-api-reference]: https://github.com/goo-yyh/openai-rs/blob/main/docs/api-reference.md
[repo-azure]: https://github.com/goo-yyh/openai-rs/blob/main/docs/azure.md
[repo-streaming]: https://github.com/goo-yyh/openai-rs/blob/main/docs/realtime-and-streaming.md
[repo-structured]: https://github.com/goo-yyh/openai-rs/blob/main/docs/structured-output-and-tools.md
[repo-provider-matrix]: https://github.com/goo-yyh/openai-rs/blob/main/docs/provider-capability-matrix.md
[repo-faq]: https://github.com/goo-yyh/openai-rs/blob/main/docs/faq.md

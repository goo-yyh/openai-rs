# API Reference

This page is a compact map of the public SDK surface: where each resource lives, how typed it is today, which feature flag gates it when applicable, and where to start with examples.

Typed level legend:

- `Strong`: primary request / response payloads are typed on the main path
- `Mixed`: core payloads are typed, with `extra` maps or raw compatibility branches retained
- `Experimental`: exported and usable, but still closer to compatibility / longtail coverage than the core resources

## Core Resources

| Resource | Entry Point | Typed Level | Feature | Example |
| --- | --- | --- | --- | --- |
| Responses | `client.responses()` | `Strong` | default | `examples/openai_responses.rs`, `examples/responses_stream.rs` |
| Chat Completions | `client.chat().completions()` | `Strong` | default | `examples/openai_chat.rs`, `examples/chat_stream.rs` |
| Legacy Completions | `client.completions()` | `Strong` | default | `examples/logprobs.rs` |
| Embeddings | `client.embeddings()` | `Strong` | default | `examples/openai_responses.rs` |
| Files | `client.files()` | `Strong` | default | `examples/files_upload.rs` |
| Uploads | `client.uploads()` | `Strong` | default | `examples/files_upload.rs` |
| Images | `client.images()` | `Mixed` | default | `examples/image_stream.rs` |
| Audio | `client.audio()` | `Mixed` | default | `examples/speech_to_text.rs`, `examples/text_to_speech.rs` |
| Moderations | `client.moderations()` | `Strong` | default | `examples/openai_responses.rs` |
| Models | `client.models()` | `Strong` | default | `examples/pagination.rs` |
| Batches | `client.batches()` | `Mixed` | default | `tests/typed_endpoints.rs`, `tests/spec_contract.rs` |
| Vector Stores | `client.vector_stores()` | `Mixed` | default | `tests/typed_endpoints.rs` |
| Fine Tuning | `client.fine_tuning()` | `Mixed` | default | `examples/fine_tuning.rs` |

## Expansion Resources

| Resource | Entry Point | Typed Level | Feature | Example / Contract |
| --- | --- | --- | --- | --- |
| Conversations | `client.conversations()` | `Mixed` | default | `tests/contract/longtail.rs` |
| Evals | `client.evals()` | `Mixed` | default | `tests/contract/longtail.rs` |
| Containers | `client.containers()` | `Mixed` | default | `tests/contract/longtail.rs` |
| Skills | `client.skills()` | `Mixed` | default | `tests/contract/longtail.rs` |
| Videos | `client.videos()` | `Mixed` | default | `tests/contract/longtail.rs` |
| Graders | `client.graders()` | `Mixed` | default | `tests/typed_endpoints.rs` |

Notes:

- These resources are supported and regression-tested.
- They still retain more compatibility surface than `responses` / `chat`, especially around `extra` maps and forward-compatible raw branches.

## Runtime / Add-on Surfaces

| Surface | Entry Point | Typed Level | Feature | Example |
| --- | --- | --- | --- | --- |
| Structured output | `parse::<T>()` builders | `Strong` | `structured-output` | `examples/parsing.rs`, `examples/responses_structured_outputs.rs` |
| Tool runner | `run_tools()` / tool helpers | `Mixed` | `tool-runner` | `examples/tool_runner.rs`, `examples/parsing_tools.rs` |
| Realtime WebSocket | `client.realtime()` / `client.ws()` | `Mixed` | `realtime` | `examples/realtime_ws.rs`, `examples/azure_realtime_ws.rs` |
| Responses WebSocket | `client.responses().ws()` | `Mixed` | `responses-ws` | `examples/responses_websocket.rs` |
| Webhooks | `client.webhooks()` | `Strong` | `webhooks` | `examples/webhook_verification.rs` |
| Azure OpenAI | `Client::builder().azure_*` | `Strong` | default | `examples/azure_chat.rs` |

## Stability Notes

- `extra: BTreeMap<String, Value>` fields are intentional forward-compatibility escape hatches and do not by themselves indicate that the main contract is untyped.
- `JsonPayload` is the common wrapper for public raw JSON compatibility surfaces such as `body_value`, `extra_body`, runtime raw branches, and provider-side body inspection.
- Experimental and longtail resources are included in CI, typed endpoint tests, fixture-based contract tests, and ecosystem smoke fixtures, but they are still more likely to expose compatibility-oriented raw branches than the core `responses` and `chat` APIs.

# Examples 索引

`examples/` 的设计目标不是把每个场景都写成大型 demo，而是给出可直接复制的最小入口。

## 基础调用

| 示例 | 说明 | feature | 常用环境变量 |
| --- | --- | --- | --- |
| `examples/openai_chat.rs` | 最小 chat completions 调用 | 默认 | `OPENAI_API_KEY` |
| `examples/openai_responses.rs` | 最小 responses 调用 | 默认 | `OPENAI_API_KEY` |
| `examples/chat_params_types.rs` | `chat-params-types` 的 Rust 等价示例 | 默认 | `OPENAI_API_KEY` |
| `examples/raw_response.rs` | 读取原始响应头和 request id | 默认 | `OPENAI_API_KEY` |
| `examples/errors.rs` | 错误分类与恢复 | 默认 | `OPENAI_API_KEY` |

## 流式与转发

| 示例 | 说明 | feature | 常用环境变量 |
| --- | --- | --- | --- |
| `examples/chat_stream.rs` | chat SSE 文本增量 | 默认 | `OPENAI_API_KEY` |
| `examples/responses_stream.rs` | responses SSE 事件流 | 默认 | `OPENAI_API_KEY` |
| `examples/stream_to_client_raw.rs` | 最朴素的上游事件透传 | 默认 | `OPENAI_API_KEY` |
| `examples/stream_to_client_sse.rs` | 更接近浏览器 / SSE Server 的转发格式 | 默认 | `OPENAI_API_KEY` |
| `examples/stream_to_client_ndjson.rs` | 适合 CLI / proxy 的 NDJSON 转发格式 | 默认 | `OPENAI_API_KEY` |

## Structured Output / Tools

| 示例 | 说明 | feature | 常用环境变量 |
| --- | --- | --- | --- |
| `examples/parsing.rs` | 结构化解析基础版 | `structured-output` | `OPENAI_API_KEY` |
| `examples/ui_generation.rs` | `ui-generation` 对应示例 | `structured-output` | `OPENAI_API_KEY` |
| `examples/parsing_tools.rs` | `parsing-tools` 对应示例 | `structured-output` | `OPENAI_API_KEY` |
| `examples/parsing_stream.rs` | 结构化流式解析 | 默认 | `OPENAI_API_KEY` |
| `examples/parsing_tools_stream.rs` | 流式解析工具参数 | 默认 | `OPENAI_API_KEY` |
| `examples/tool_runner.rs` | `runTools()` 风格工具执行 | `tool-runner` | `OPENAI_API_KEY` |
| `examples/function_call.rs` | 函数调用基础版 | 默认 | `OPENAI_API_KEY` |
| `examples/function_call_stream.rs` | 函数调用流式版 | 默认 | `OPENAI_API_KEY` |
| `examples/function_call_stream_raw.rs` | 原始函数调用增量 | 默认 | `OPENAI_API_KEY` |

## WebSocket / Realtime

| 示例 | 说明 | feature | 常用环境变量 |
| --- | --- | --- | --- |
| `examples/realtime_ws.rs` | OpenAI Realtime WebSocket | `realtime` | `OPENAI_API_KEY` |
| `examples/azure_realtime_ws.rs` | Azure Realtime WebSocket | `realtime` | `AZURE_OPENAI_*` |
| `examples/responses_websocket.rs` | Responses WebSocket | `responses-ws` | `OPENAI_API_KEY` |

## Provider / 其他主题

| 示例 | 说明 | feature | 常用环境变量 |
| --- | --- | --- | --- |
| `examples/azure_chat.rs` | Azure OpenAI 最小示例 | 默认 | `AZURE_OPENAI_*` |
| `examples/files_upload.rs` | 文件上传 | `multipart` | `OPENAI_API_KEY` |
| `examples/audio_roundtrip.rs` | 音频输入输出 | 默认 | `OPENAI_API_KEY` |
| `examples/webhook_verification.rs` | webhook 验签 | `webhooks` | `OPENAI_WEBHOOK_SECRET` |
| `examples/fine_tuning.rs` | fine-tuning | 默认 | `OPENAI_API_KEY` |

## 运行建议

- 编译所有示例：`cargo check --examples --all-features`
- 带 feature 运行：`cargo run --example ui_generation --features structured-output`
- Realtime 示例建议单独运行，便于观察事件流和握手失败信息

# 流式与 Realtime 说明

`openai-rs` 当前提供三类增量能力：

- HTTP SSE：`chat.completions().stream()`、`responses().stream()`
- Realtime WebSocket：`client.realtime().ws()`
- Responses WebSocket：`client.responses().ws()`

runtime event 的边界语义见 [runtime-event-contract.md](./runtime-event-contract.md)。

## SSE

SSE 适合标准请求-响应模型下的增量输出。

```rust,ignore
use futures_util::StreamExt;

let mut stream = client
    .responses()
    .stream()
    .model("gpt-5.4")
    .input_text("hello")
    .send()
    .await?;

while let Some(event) = stream.next().await {
    println!("{:?}", event?);
}
```

## Realtime WebSocket

需要开启 `realtime` feature。

```rust,ignore
use futures_util::StreamExt;
use openai_rs::SocketStreamMessage;

let socket = client
    .realtime()
    .ws()
    .model("gpt-4o-realtime-preview")
    .connect()
    .await?;

let mut events = socket.stream();
while let Some(event) = events.next().await {
    match event {
        SocketStreamMessage::Message(message) => println!("{message:?}"),
        SocketStreamMessage::Error(error) => {
            eprintln!("{error}");
            break;
        }
        SocketStreamMessage::Close => break,
        _ => {}
    }
}
```

## 事件类型

当前 Realtime / Responses WebSocket 事件已经升级为 enum：

- `RealtimeServerEvent`
- `ResponsesServerEvent`

已对常见事件提供强类型变体：

- `response.created`
- `response.output_text.delta`
- `session.created`（Realtime）

未知事件会进入 `Unknown(...)`，以保证向前兼容。

## 错误分类

`WebSocketError` 现在会区分：

- `transport`
- `protocol`
- `server`

这有助于快速判断是网络层问题、事件解码问题，还是服务端主动推送的错误。

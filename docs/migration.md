# 迁移说明

## 从早期 `0.1.x` 到当前优化版本

这次优化主要集中在公开库边界和 feature 设计，最明显的变化如下。

## 默认 feature 变轻

默认只启用：

- `stream`
- `multipart`
- `webhooks`
- `rustls-tls`

以下能力需要显式开启：

- `structured-output`
- `tool-runner`
- `realtime`
- `responses-ws`

如果你的代码此前直接使用这些能力，现在需要在 `Cargo.toml` 中补 feature。

## WebSocket API 改为按 feature 公开

不开 `realtime` / `responses-ws` 时：

- `client.realtime().ws()`
- `client.responses().ws()`
- `RealtimeSocket`
- `ResponsesSocket`

都不会再出现在公开 API 中。

## WebSocket 事件改为 enum

此前 Realtime / Responses 事件更接近原始 map。

现在会返回：

- `RealtimeServerEvent`
- `ResponsesServerEvent`

调用方应改为匹配 enum 变体，而不是直接读取 `event_type` 字段。

## 新增更明确的缺参错误

部分常见请求缺参现在会返回 `Error::MissingRequiredField`，而不再统一落到 `InvalidConfig`。

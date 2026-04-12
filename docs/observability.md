# 可观测性说明

`openai-core` 当前使用 `tracing` 暴露关键运行时字段。

## HTTP 请求

关键字段包括：

- `endpoint_id`
- `provider`
- `attempt`
- `max_retries`
- `delay_ms`

这些字段主要出现在 transport 层的重试与请求执行路径中。

## WebSocket

关键字段包括：

- `url`
- `endpoint_id`
- `event_type`
- `code`

这些字段主要出现在：

- 建立连接
- 收到事件
- 发送消息
- 主动关闭

## 建议

- 在应用侧统一初始化 `tracing-subscriber`
- 在生产环境把 request / websocket 相关 span 与应用层 request id 关联起来
- 对 `WebSocketErrorKind::server` 单独做报警与统计

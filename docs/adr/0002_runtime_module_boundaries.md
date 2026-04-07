# ADR 0002: runtime 与 resources 模块边界收敛

## 状态

Accepted

## 背景

Phase 3 之后，`openai-rs` 的大部分 namespace 已经拆出独立文件，但仍有三个维护成本较高的聚合点：

- `src/resources/mod.rs` 同时承载公开类型、namespace handle、chat builder、responses builder
- `src/stream/mod.rs` 同时承载 SSE 协议、chat runtime、responses runtime、assistants runtime
- `src/websocket` 已拆出事件与 transport，但缺少显式模块边界文件

这会带来三个问题：

- review 时很难只看某个 namespace / runtime
- 新增流式协议逻辑容易继续堆回单文件
- 内部共享层不清晰，重复 builder / merge helper 容易再次出现

## 决策

本轮把边界固定为：

- `resources/chat.rs`
  包含 chat namespace、builder、structured output、tool runner
- `resources/responses.rs`
  包含 responses / realtime namespace 与对应 builder
- `resources/common.rs`
  负责通用 request builder 与 typed request state
- `stream/sse.rs`
  只负责行解码和 SSE 事件流
- `stream/partial_json.rs`
  只负责 partial JSON 容错解析
- `stream/chat.rs`
  只负责 chat 流聚合与 runtime event
- `stream/responses.rs`
  只负责 responses 流聚合与 runtime event
- `stream/assistant.rs`
  只负责 assistants / beta threads 流聚合与 runtime event
- `stream/value_helpers.rs`
  只负责流聚合内部使用的 `serde_json::Value` merge helper
- `websocket/mod.rs`
  只负责暴露 `events` 与 `core` 两层

## 结果

- `resources/mod.rs` 回到“公开类型 + handle + re-export”职责
- `stream/mod.rs` 回到“模块边界 + re-export”职责
- chat / responses / assistant / websocket 可以按 runtime 单独 review
- 新增协议逻辑时，默认先落到对应 runtime 子模块，而不是回填聚合文件

## 后续动作

- 继续观察 `resources/longtail.rs` 是否需要按 namespace 再拆一轮
- 如果 websocket 事件类型继续增长，可把 `events.rs` 再按 realtime / responses 划分

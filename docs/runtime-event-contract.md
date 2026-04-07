# Runtime Event Contract

本文档约束 `openai-rs` 当前三类 runtime event stream 的行为：

- `ChatCompletionEventStream`
- `ResponseEventStream`
- `AssistantEventStream`

目标不是重述所有事件枚举，而是固定这些流在边界场景下的可依赖语义。

## 通用约定

- runtime event 按线上的原始事件顺序发出，不做重排。
- 每次发出高层 runtime event 之前，内部快照都会先应用当前增量；因此事件里携带的 `snapshot` 总是“包含本次增量之后”的状态。
- `final_*()` 返回的是流消费完成后的最后快照，不要求和中途任意单个 runtime event 一一对应，但必须和整条流最终累计结果一致。
- partial JSON 字段采用“可恢复则解析、明显非法则不解析”的策略：
  - 例如 `{"city":"Sha` 会得到 `parsed = {"city":"Sha"}`
  - 例如 `{"city":}` 会得到 `parsed = None`

## Chat

- `ContentDelta` / `ToolCallArgumentsDelta` / `LogProbs*Delta` 的 `snapshot` 都是累计值，不是仅当前片段。
- `parsed` / `parsed_arguments` 只反映当前累计字符串可推导出的 JSON；如果片段非法，会回落为 `None`。
- `ContentDone` / `ToolCallArgumentsDone` / `LogProbs*Done` 在同一个 choice 的同一条完成路径上只发一次。
- `final_chat_completion()`、`final_message()`、`final_content()` 都基于同一份最终累计快照。

## Responses

- `OutputTextDelta` / `OutputTextDone` 的 `snapshot` 表示当前累计文本。
- `FunctionCallArgumentsDelta` 的 `snapshot` 表示当前累计参数字符串。
- 当 `response.output_item.added`、`response.content_part.added` 晚于对应 delta 到达时，SDK 会把先前累计的文本/参数回填进响应快照；因此乱序到达不会清空已经累计的内容。
- `Completed(Response)` 直接携带当下的响应快照。
- 如果服务端重复发送 `response.completed`，runtime stream 会按线序重复发出 `Completed`；但 `final_response()` 仍然收敛到最后一份稳定快照。

## Assistants

- `AssistantRuntimeEvent::Event` 始终保留原始 SSE 事件。
- `thread.message.delta` 和 `thread.run.step.delta` 会直接合并到 `AssistantStreamSnapshot`；即使对应的 `created` 事件稍后才到，最终快照仍然会收敛。
- `TextCreated` / `TextDelta` / `TextDone` 与 `ToolCallCreated` / `ToolCallDelta` / `ToolCallDone` 都基于当前合并后的 message / run step 快照派生。
- `final_snapshot()` 在原始流和 `events()` 包装流上应得到等价的最终 message / run / run step 状态。

# 0007 Live Provider 实测结论

## 背景

这轮工作不是静态对照，也不是 mock contract 检查，而是基于真实 Provider API key 对 `openai-rs` 的 live tests 做了一次扩充和实跑验证。

目标有两类：

- 验证三家已接入 Provider 在真实线上环境中的主链路能力是否可用
- 把后续仍值得继续补强的测试点沉淀下来，作为下一阶段优化清单

本轮使用的是本地临时环境变量：

- 智谱 `Zhipu`
- `MiniMax`
- `ZenMux`

这些 key 只在本地 shell 会话中使用，没有写入仓库文件，也没有提交上传。

## 本轮范围

本轮实测主要覆盖了下面这些能力：

- Chat completion 基础请求
- Chat completion SSE 流式输出
- Chat 结构化 JSON 输出
- Chat function/tool calling
- Responses 文本输出
- Responses 结构化 JSON 输出
- Invalid model 错误形态
- 模型列表与模型自动探测

对应测试文件：

- `tests/provider_live.rs`
- `tests/provider_live/common.rs`
- `tests/provider_live/zhipu.rs`
- `tests/provider_live/minimax.rs`
- `tests/provider_live/zenmux.rs`

另外顺手补了一处错误提取逻辑：

- `src/transport/mod.rs`

新增了对以下错误包裹风格的 message 提取：

- 顶层 `error: "..."` 字符串
- `base_resp.status_msg`

## 执行结果

本轮真实执行通过：

```bash
cargo test --test provider_live -- --ignored --nocapture
```

结果：

- `19 passed`
- `0 failed`

同时额外验证通过：

```bash
cargo test --lib extract_top_level_error_string_message
cargo test --lib extract_minimax_style_status_message
cargo test --test contract --no-run
```

## 覆盖矩阵

### 1. Zhipu

本轮实测通过 6 条 live tests：

- `chat_completion_basic`
- `chat_completion_stream_basic`
- `chat_structured_json_output`
- `chat_tool_calling`
- `responses_text_or_provider_error_shape`
- `invalid_model_error_shape`

结论：

- `chat` 基础调用可用
- `chat` 流式调用可用
- 结构化 JSON 输出可用
- 强制工具调用可用
- `responses` 当前真实行为是 `404 NotFound`
- SDK 能把无效模型错误标准化为 `400 BadRequest`

实测观察：

- 结构化输出直接返回 `{"city":"Paris","country":"France"}`
- 工具调用参数成功返回 `{"a":2,"b":3}`
- `responses` 路径返回 `404 Not Found`
- 无效模型返回 `400`，错误消息为“模型不存在，请检查模型代码。”

### 2. MiniMax

本轮实测通过 6 条 live tests：

- `chat_completion_basic`
- `chat_completion_stream_basic`
- `chat_structured_json_output`
- `chat_tool_calling`
- `responses_text_or_provider_error_shape`
- `invalid_model_error_shape`

结论：

- `chat` 基础调用可用
- `chat` 流式调用可用
- 结构化 JSON 输出可用
- 强制工具调用可用
- `responses` 当前真实行为是 `404 NotFound`
- SDK 能把无效模型错误标准化为 `400 BadRequest`

实测观察：

- 流式输出最终可见文本是 `你好！`
- 结构化输出直接返回 `{"city":"Paris","country":"France"}`
- 工具调用参数成功返回 `{"a": 2, "b": 3}`
- `responses` 路径返回 `404 page not found`
- 无效模型返回 `400`，错误消息中带 `unknown model`

需要注意的一点：

- `MiniMax` 有时会在原始输出中带 `<think>...</think>` 推理内容
- 当前 live tests 会先做“可见文本净化”再断言，因此主链路可用性没有问题
- 但如果未来要把“严格输出质量”作为更高标准，这一类推理泄漏仍值得单独跟踪

### 3. ZenMux

本轮实测通过 7 条 live tests：

- `models_list`
- `chat_completion_with_discovered_model`
- `chat_structured_json_output`
- `chat_tool_calling`
- `responses_text_output`
- `responses_structured_json_output`
- `invalid_model_error_shape`

结论：

- `models.list` 可用
- `chat` 基础调用可用
- 结构化 JSON 输出可用
- 强制工具调用可用
- `responses` 可用，但不是所有模型都支持
- SDK 能把无效模型错误标准化为 `404 NotFound`

实测观察：

- `models.list` 返回 `123` 个模型
- `chat` 默认发现到了 `openai/gpt-4.1-nano`
- `chat` 基础输出返回 `OK`
- `chat` 工具调用参数成功返回 `{"a":2,"b":3}`
- `responses` 模型探测过程中：
  - `openai/gpt-4.1` 返回 `No provider available`
  - `openai/gpt-4.1-mini` 不支持 `/v1/responses`
  - `openai/gpt-4.1-nano` 不支持 `/v1/responses`
  - `openai/gpt-4o` 不支持 `/v1/responses`
  - `openai/gpt-4o-mini` 不支持 `/v1/responses`
  - 最终探测到 `openai/gpt-5` 可用于 `/v1/responses`
- `responses` 文本输出返回 `OK`
- `responses` 结构化输出返回 `{"city":"Paris","country":"France"}`
- 无效模型返回 `404`，消息为 `Requested model is not valid`

## 这轮结论

沿着“真实可用性”这条线看，本轮结论可以整理成下面几条：

- `Zhipu` 和 `MiniMax` 当前可以稳定支持 `chat / stream / structured output / tool calling`
- `Zhipu` 和 `MiniMax` 当前在真实线上环境里并不支持本 SDK 里的 `responses` 入口，至少按这轮实测结果会返回标准化 `404`
- `ZenMux` 的 `chat` 和 `responses` 都可以工作，但 `responses` 对模型有额外限制，必须做模型探测或显式指定
- `openai-rs` 在这三家 Provider 上的“请求发送、JSON 反序列化、stream 聚合、tool call 参数解析、错误标准化”主链路已经具备真实可用性
- live tests 已经不再只是“请求不报错就算过”，而是开始校验输出语义、JSON 字段和值、工具参数和错误形态

## 当前仍需优化的点

下面这些不再是“功能有没有”的问题，而是后续非常值得继续优化的测试点。

### P0 测试稳定性

- 给 live tests 增加分层执行策略
  现在全部 live tests 一次性跑完约两分钟，已经不算轻。后续建议拆成：
  - `smoke`
  - `extended`
  - `slow`

- 给 `ZenMux responses` 模型探测增加缓存
  现在每次运行都会重新探测一次可用模型，虽然可行，但成本偏高，也引入额外波动。

- 把 provider 探测与真正断言拆开
  尤其是 `ZenMux`，模型探测失败和接口失败是两类问题，后续应让测试输出更明确。

- 为 live tests 增加 request id 打印
  当前失败时还缺请求追踪字段，真实线上排查时信息不够。

### P0 能力矩阵表达

- 把 `responses` 能力从“代码里标记支持/不支持”升级成“实测矩阵”
  当前 SDK 内部 capability 是静态的，但实测表明：
  - `Zhipu` / `MiniMax` 的 `responses` 在真实线上不可用
  - `ZenMux` 的 `responses` 只对部分模型可用
  这意味着未来最好维护一份“Provider x 能力 x 模型”的实测矩阵，而不是只靠静态布尔值。

- 增加“预期不支持”的显式 live tests 分类
  比如：
  - `expected_unsupported`
  - `expected_provider_error`
  这样可以把“不支持”和“真正回归失败”区分开。

### P1 输出质量断言

- 把当前关键词断言升级成更严格的质量约束
  例如：
  - 限制回答语言
  - 限制句数
  - 限制不得出现额外解释
  - 限制不得出现 markdown 代码块

- 对 `MiniMax` 的 `<think>` 泄漏做单独断言
  当前已经做了净化，但未来更好的方式是：
  - 显式验证原始输出是否含推理块
  - 如果能通过 provider 参数关闭，就补一条“关闭推理泄漏”的 live test

- 对 structured output 增加 schema 严格断言
  当前只校验字段和值，后续可以补：
  - 无额外字段
  - 类型完全匹配
  - key 顺序不依赖 markdown 或自然语言包装

### P1 Tool Calling 深度

- 当前只验证了“首轮工具调用参数是否正确”
  后续还应补：
  - 多个工具调用
  - 多轮工具调用
  - tool result 回填后的最终 assistant 总结
  - 流式 tool arguments 增量正确拼接

- 补 `run_tools()` 的真实 live tests
  现在实测的是“模型是否真的发起 tool call”，还没有把 SDK 自带 `tool-runner` 走一遍真实线上全链路。

### P1 Responses 深度

- `ZenMux` 只测了 responses 文本和 JSON 输出
  后续可以继续补：
  - `responses.stream()`
  - `response.function_call_arguments.delta`
  - `response.output_text.delta`
  - `response.completed` 的最终聚合一致性

- 如果未来 `Zhipu` / `MiniMax` 开始支持 `responses`
  需要立刻把当前“错误形态测试”升级成真正的成功路径 live tests。

### P1 错误与限流

- 目前没有真实触发 `429 rate limit`
  这是有意的，因为直接在线上刷请求不安全、不可控，也容易污染账号额度。

- 后续建议补两类测试：
  - 本地 contract/replay fixture：覆盖 provider 特有的 429 错误包裹格式
  - 非生产节流环境：如果未来有 sandbox 或受控网关，再做真实 429 live test

- 需要补更多错误形态
  例如：
  - 鉴权失败 `401`
  - 权限不足 `403`
  - 无效参数 `422`
  - 上游 provider 不可用导致的 `502/503`

### P2 工程化

- 为 live tests 增加结果落盘
  比如把：
  - 实测模型
  - 请求耗时
  - request id
  - provider message
  落到本地报告文件，便于做长期对比。

- 增加 nightly 或手动触发的 live test workflow 方案
  但需要确保：
  - key 不进仓库
  - 默认 CI 不自动跑线上付费测试
  - 额度和速率有明确上限

## 建议的后续顺序

如果按收益排序，后续最值得继续做的是：

1. 把 `run_tools()` 的真实 live tests 补上
2. 给 `responses.stream()` 增加真实 Provider live tests
3. 把 `ZenMux responses` 的模型探测缓存和报告落盘补上
4. 增加更严格的输出质量断言
5. 补充 `401/403/422/502` 的 provider-specific 错误形态 contract tests

## 最终判断

这轮之后，`openai-rs` 的 live provider 测试已经从“非常薄的 smoke test”升级成了一套更接近真实使用场景的验证集。

当前可以认为：

- `Zhipu`、`MiniMax`、`ZenMux` 三家的 chat 主链路已经完成了真实线上验证
- `tool calling` 和 `structured output` 不再只是本地 mock 通过，而是已经拿真实接口验证过
- `ZenMux` 的 `responses` 能力也已经真实跑通
- `Zhipu` / `MiniMax` 当前对 `responses` 的真实行为已被明确记录为“不支持或至少当前路径不可用”

后续继续优化的重点，不是再去补“有没有这个入口”，而是把 live tests 做得更稳定、更可解释、更接近长期公开 SDK 的质量基线。

# FAQ

## `openai-core` 是官方 SDK 吗？

不是。它是社区维护的 Rust SDK，但目标是尽量贴近 OpenAI 官方接口与生态习惯。

## 默认为什么不直接开启全部 feature？

为了让基础依赖更轻，也让库使用者只为自己真的要用的能力付费。

默认只保留最常见的 HTTP / SSE / multipart / webhook 链路；`structured-output`、`tool-runner`、`realtime`、`responses-ws` 都需要显式开启。

## 应该优先用 Chat 还是 Responses？

- 如果你在做传统 chat completions 迁移，先用 `chat().completions()`
- 如果你希望贴近 OpenAI 新能力演进，优先评估 `responses()`
- 如果你需要跨 provider 兼容，建议分别验证两条链路，不要假设 provider 都完整支持 `/v1/responses`

## 为什么 WebSocket API 看起来更“强类型”了？

因为 phase 4 之后，Realtime / Responses WebSocket 事件已经升级为 enum。这样做是为了让上层代码能直接 `match` 事件，而不是到处解析裸 JSON。

## 如何判断某次改动是不是 breaking change？

先看 `bash ./scripts/check-public-api.sh` 是否变化，再结合以下三点判断：

- 调用方导入路径是否变化
- 公开类型、字段、方法、feature 是否被移除
- README / migration 文档是否需要同步修改

## live tests 默认为什么是 `#[ignore]`？

因为它们依赖真实 provider 凭据，而且会产生费用或速率限制风险。仓库内默认只把它们作为手动验证路径。

## 哪里可以快速找到对应示例？

- 示例目录索引：`docs/examples.md`
- provider 差异：`docs/provider-capability-matrix.md`
- 迁移影响：`docs/migration.md`

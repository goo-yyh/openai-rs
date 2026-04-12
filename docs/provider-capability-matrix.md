# Provider 能力矩阵

这张表描述的是 `openai-core` 当前对各 provider 的 SDK 支持和验证策略，不代表第三方 provider 一定完整实现了所有 OpenAI 能力。

| Provider | 接入方式 | Chat / SSE | Responses | Realtime / WebSocket | live workflow | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| OpenAI | 一等支持 | 一等支持 | 一等支持 | 一等支持 | 已集成 | 主兼容目标，README / examples / CI 默认围绕它设计 |
| Azure OpenAI | 一等支持 | 一等支持 | 一等支持 | Realtime 已适配 | 未单独集成 | 依赖 `azure_endpoint`、deployment 与 `api-version` |
| Zhipu | 兼容支持 | 已有 live 覆盖 | provider 实现决定最终行为 | 未作为主路径提供 | 已集成 | 重点验证 chat、stream、tool calling、错误归一化 |
| MiniMax | 兼容支持 | 已有 live 覆盖 | provider 实现决定最终行为 | 未作为主路径提供 | 已集成 | 对思维链泄漏、错误形态和 Responses 兼容性做了单独观察 |
| ZenMux | 兼容支持 | 已有 live 覆盖 | 通过模型探测支持 | 未作为主路径提供 | 已集成 | 会先探测可用模型，再验证 Responses 与流式链路 |
| Custom Provider | 扩展支持 | 取决于接入方 | 取决于接入方 | 取决于接入方 | 无 | 建议先从 chat / responses 文本链路做兼容回归 |

## 如何解读

- “一等支持”表示仓库会把该 provider 当成长期维护目标，文档、示例和主要回归都优先覆盖
- “兼容支持”表示 SDK 提供稳定接入层，但最终能力取决于上游 provider 对 OpenAI 协议的实现深度
- `live workflow` 只代表仓库内已集成的手动线上验证，不代表没有该 workflow 的 provider 就不支持

## 推荐实践

- OpenAI / Azure：优先作为主生产路径
- Zhipu / MiniMax / ZenMux：上线前至少执行一次手动 live workflow
- 自定义 provider：先验证鉴权、路径重写、错误形态和流式事件顺序

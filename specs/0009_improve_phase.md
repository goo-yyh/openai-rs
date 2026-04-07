# 0009: `openai-rs` 可实施优化分期方案

## 1. 文档目标

本文档基于 [0008.md](./0008.md) 的再次评审结果，给出一份可以直接落地执行的优化方案。

目标不是泛泛而谈“继续优化”，而是明确：

- 先做什么
- 后做什么
- 每个阶段改哪些东西
- 每个阶段如何验收
- 每个阶段结束后，`openai-rs` 能提升到什么状态

本文将默认：

- `openai-rs` 当前已经“可用”
- 当前主要短板在测试、工程化、长尾强类型、结构可维护性
- 优化过程必须保持每个 phase 都处于“可合并、可发布、可验证”的状态

## 2. 基线结论

来自 [0008.md](./0008.md) 的当前基线：

- 综合评分：`83 / 100`
- 功能覆盖：`92 / 100`
- 主链路可用性：`88 / 100`
- Provider / Azure：`90 / 100`
- 测试广度：`72 / 100`
- 测试深度：`78 / 100`
- 代码质量：`84 / 100`
- 可维护性：`80 / 100`
- 工程化与发布成熟度：`71 / 100`
- 公开库人体工学：`82 / 100`

当前最关键的改进目标，不是继续堆主功能，而是提升这四项：

1. 测试广度与深度
2. 长尾资源强类型程度
3. 模块结构可维护性
4. 发布工程化成熟度

## 3. 执行原则

### 3.1 每个 phase 都必须可单独合并

不做“大爆炸式重构”。

每个 phase 都应满足：

- `cargo build`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo deny check`
- `cargo check --examples --all-features`

### 3.2 先补验证能力，再动大结构

因为当前最大风险不是“功能不存在”，而是“重构后不好确认是否回归”。

因此顺序必须是：

1. 先补测试和 CI
2. 再做强类型和拆模块
3. 最后做发布与长期维护工程化

### 3.3 每个 phase 都要收敛范围

不能把这些动作混在同一阶段：

- 大量资源强类型化
- `resources` 大拆分
- CI / release 重构
- examples / README 全面重写

这些动作相互耦合太高，会放大回归风险。

### 3.4 优先做高收益区域

优先级按影响排序：

1. 主链路测试
2. 长尾资源强类型
3. 发布工程化
4. 文档和 examples 再补齐

## 4. 总体路线

建议分为 6 个 phase。

### 路线概览

| Phase | 主题 | 目标 |
| --- | --- | --- |
| Phase 0 | 基线固化 | 先把“怎么衡量进步”定义清楚 |
| Phase 1 | 测试补强 | 把“能用”提升到“能稳定回归” |
| Phase 2 | 流式与协议硬化 | 把最脆弱的 SSE / WS / parser 打牢 |
| Phase 3 | 长尾资源强类型化 | 把功能齐提升为 API 质量齐 |
| Phase 4 | 结构与可维护性优化 | 把大文件和聚合模块进一步拆开 |
| Phase 5 | 发布工程化与文档收尾 | 把库做成可长期公开维护的状态 |

## 5. Phase 0：基线固化

### 目标

把本轮优化的评估基线、覆盖矩阵和验收方式固定下来，避免后面每个阶段都在重新定义“什么算完成”。

### 范围

- 固化对照基线
- 建立测试主题矩阵
- 建立 examples 覆盖矩阵
- 建立里程碑看板文档

### 具体任务

1. 新增 `specs/coverage_matrix.md` 或类似文档，内容包括：
   - `openai-node/tests` 主题清单
   - `openai-rs/tests` 当前对应项
   - 缺失项

2. 新增 `specs/examples_matrix.md` 或类似文档，内容包括：
   - `openai-node/examples` 到 `openai-rs/examples` 的映射
   - 已覆盖
   - 部分覆盖
   - 未覆盖

3. 在 `0009_improve_phase.md` 中固定每个 phase 的验收标准与分数目标。

4. 给当前最重要的技术债建立追踪表：
   - `JsonRequestBuilder<Value>` 暴露点
   - `resources/mod.rs` 与其他大型模块体量
   - 缺失测试专题

### 建议涉及文件

- [specs](/Users/yuyuehui/open-ai/openai-rs/specs)
- [README.md](/Users/yuyuehui/open-ai/openai-rs/README.md)
- [examples](/Users/yuyuehui/open-ai/openai-rs/examples)
- [tests](/Users/yuyuehui/open-ai/openai-rs/tests)

### 交付物

- 一份测试覆盖矩阵
- 一份 examples 覆盖矩阵
- 一份技术债清单

### 验收标准

- 能准确回答“还缺哪些测试主题”
- 能准确回答“还缺哪些 examples”
- 后续 phase 的范围不再模糊

### 非目标

- 本 phase 不做大量代码改动
- 不做功能实现

### 完成后预期

- 不直接提升功能分
- 但会显著降低后续 phase 的执行混乱度

## 6. Phase 1：测试补强

### 目标

把 `openai-rs` 从“主链路有测试”提升到“主要风险点都有独立测试专题”。

这是整个路线里优先级最高的一阶段。

### 范围

- contract tests 细化
- snapshot tests 补齐
- parser / retry / logger / path / upload / Azure 专项测试
- live tests workflow 设计

### 具体任务

1. 新增这些专题测试文件：

- `tests/retry_timeout.rs`
- `tests/logger.rs`
- `tests/path_query.rs`
- `tests/uploads.rs`
- `tests/parser.rs`
- `tests/azure.rs`

2. 把当前集中在 [contract/resources.rs](/Users/yuyuehui/open-ai/openai-rs/tests/contract/resources.rs) 里的部分主题拆出来，降低单文件复杂度。

3. 重点补这些场景：

- `retry-after`
- `retry-after-ms`
- timeout retry
- `OPENAI_LOG`
- 自定义 logger
- `send_with_meta()` / `send_raw()`
- path segment 编码
- query 编码
- `to_file()` 各输入路径
- multipart body 与文件混合
- Azure endpoint / deployment / bearer auth / realtime 路径

4. 给流式 partial JSON 解析补单独测试集，而不是只依附在 contract tests 中。

5. 新增 GitHub Actions 手动触发 live workflow：
   - `workflow_dispatch`
   - 按 provider 分 job
   - 不默认在每个 PR 上跑

### 建议涉及文件

- [tests](/Users/yuyuehui/open-ai/openai-rs/tests)
- [src/transport/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/transport/mod.rs)
- [src/client.rs](/Users/yuyuehui/open-ai/openai-rs/src/client.rs)
- [src/files/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/files/mod.rs)
- [src/stream/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/stream/mod.rs)
- [.github/workflows](/Users/yuyuehui/open-ai/openai-rs/.github/workflows)

### 交付物

- 多个专题测试文件
- live tests 手动 workflow
- 测试覆盖矩阵同步更新

### 验收标准

- 新增的专题测试可以独立执行
- 主要 transport / parser / upload / path 风险点都有针对性测试
- CI 中可以区分：
  - 快速测试
  - 全量测试
  - 手动 live 测试

### 评分目标

- 测试广度：`72 -> 82+`
- 测试深度：`78 -> 84+`

### 风险

- 测试拆分本身会暴露隐藏耦合，短期内可能需要顺手修若干实现问题

## 7. Phase 2：流式与协议硬化

### 目标

针对最容易出边界问题的运行时层，系统补强：

- SSE
- partial JSON
- Responses runtime event
- Assistant stream
- Realtime / Responses WebSocket

### 范围

- 协议边界测试
- 异常顺序测试
- decoder 容错
- 流式快照一致性验证

### 具体任务

1. 新增或补强以下测试主题：

- 空 `data:` / 多行 `data:`
- 事件乱序
- 重复 `completed`
- `function_call_arguments.delta` 分段拼接
- `output_text.delta` 与最终聚合一致
- `final_response()` / `final_snapshot()` 一致性
- 非法 JSON 片段与可恢复 JSON 片段
- WebSocket 握手失败
- WebSocket 关闭码和错误映射

2. 为 [src/stream/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/stream/mod.rs) 中的复杂聚合逻辑补更多内部单测。

3. 为 [src/websocket/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/websocket/mod.rs) 增加更多协议层快照测试和错误分支测试。

4. 把 Responses / Assistants / Chat 的 runtime event 行为写成更明确的契约文档。

### 建议涉及文件

- [src/stream/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/stream/mod.rs)
- [src/websocket/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/websocket/mod.rs)
- [tests/snapshots.rs](/Users/yuyuehui/open-ai/openai-rs/tests/snapshots.rs)
- [tests/websocket.rs](/Users/yuyuehui/open-ai/openai-rs/tests/websocket.rs)

### 交付物

- 新的协议边界测试
- 新的快照
- runtime event 契约说明

### 验收标准

- SSE / WS / partial JSON 的主要边界都可回归
- 流式累计结果与最终快照结果一致
- WebSocket 失败路径不再只靠手工验证

### 评分目标

- 流式 / Realtime / Tool Runner：`87 -> 91+`
- 测试深度：`84 -> 87+`

### 风险

- 这阶段容易发现真实设计问题，不要把“补测试”和“大改 API”绑在一起

## 8. Phase 3：长尾资源强类型化

### 目标

把“功能存在但靠 `Value` builder 使用”的资源，逐步提升为更稳定、更易发现的公开 API。

### 范围

- 高价值长尾资源
- 请求参数类型
- 响应结构类型
- 公开 builder 一致性

### 优先级顺序

建议按收益排序，而不是按 namespace 名字排序：

1. `images`
2. `audio`
3. `fine_tuning`
4. `batches`
5. `conversations`
6. `evals`
7. `containers`
8. `skills`
9. `videos`

### 具体任务

1. 为每个 namespace 明确：
   - 最常用请求类型
   - 最常用响应类型
   - 保留 `body_value()` 作为逃生舱的范围

2. 逐批替换 `JsonRequestBuilder<Value>` 暴露点：
   - 先做 create / retrieve / list 这类高频接口
   - 再做 edit / action / run 这类长尾接口

3. 统一 builder 习惯：
   - `.model(...)`
   - `.input_text(...)`
   - `.metadata(...)`
   - `.timeout(...)`
   - `.send()` / `.send_with_meta()` / `.send_raw()`

4. 每做完一个 namespace，同步补：
   - contract tests
   - examples
   - README 对应章节

### 建议涉及文件

- [src/resources](/Users/yuyuehui/open-ai/openai-rs/src/resources)
- [src/lib.rs](/Users/yuyuehui/open-ai/openai-rs/src/lib.rs)
- [tests/contract](/Users/yuyuehui/open-ai/openai-rs/tests/contract)
- [examples](/Users/yuyuehui/open-ai/openai-rs/examples)

### 交付物

- 一批强类型请求与响应
- 对应 contract tests
- 对应 examples

### 验收标准

- `JsonRequestBuilder<Value>` 暴露点显著下降
- 高价值资源不再要求用户大量手写 `serde_json::json!`
- 文档与 examples 同步跟上

### 建议量化目标

- 第一轮目标：`62` 处降到 `35` 以下
- 第二轮目标：降到 `20` 以下

### 评分目标

- 功能覆盖：`92 -> 95+`
- 公开库人体工学：`82 -> 89+`
- 可维护性：`80 -> 84+`

### 风险

- 这阶段最容易引入 public API 变化，必须和 `public-api` 基线联动

## 9. Phase 4：结构与可维护性优化

### 目标

把当前仍偏集中的实现进一步拆开，让代码库适合长期演进。

### 范围

- `resources` 目录继续拆分
- 大文件拆分
- 内部公共逻辑下沉
- 模块职责边界收清

### 具体任务

1. 优先继续拆这些文件：

- [src/resources/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/resources/mod.rs)
- [src/stream/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/stream/mod.rs)
- [src/websocket/mod.rs](/Users/yuyuehui/open-ai/openai-rs/src/websocket/mod.rs)

2. 建立更清晰的内部目录边界，例如：

```text
src/resources/
├── common/
├── chat.rs
├── responses.rs
├── images.rs
├── audio.rs
├── files.rs
├── uploads.rs
├── batches.rs
├── conversations.rs
├── evals.rs
├── containers.rs
├── skills.rs
├── videos.rs
└── beta/
```

3. 提炼内部共享层：

- request builder 公共层
- multipart helper
- SSE / stream 聚合 helper
- websocket 握手与事件解码 helper

4. 对复杂模块补简短 ADR 或模块级注释，记录设计边界。

### 建议涉及文件

- [src/resources](/Users/yuyuehui/open-ai/openai-rs/src/resources)
- [src/stream](/Users/yuyuehui/open-ai/openai-rs/src/stream)
- [src/websocket](/Users/yuyuehui/open-ai/openai-rs/src/websocket)
- [docs/adr](/Users/yuyuehui/open-ai/openai-rs/docs/adr)

### 交付物

- 更细的模块结构
- 更明确的内部共享层
- 新的 ADR 或模块设计文档

### 验收标准

- 大文件体量明显下降
- 新增资源或改动局部逻辑时，不需要触碰过多无关文件
- 代码审查能按 namespace 或 runtime 模块拆开进行

### 评分目标

- 可维护性：`84 -> 89+`
- 代码质量：`84 -> 88+`

### 风险

- 这是机械性较强的重构，必须放在测试补强之后做

## 10. Phase 5：发布工程化与文档收尾

### 目标

把当前“有 CI”的状态，提升为“具备长期公开发布能力”的状态。

### 范围

- 发布 workflow
- 多平台矩阵
- semver / public API 门禁
- docs / examples 收尾
- live tests 集成策略

### 具体任务

1. 新增 release workflow：
   - `cargo publish --dry-run`
   - 版本检查
   - changelog 检查
   - docs.rs 构建检查

2. 新增更完整的 CI matrix：
   - Linux
   - macOS
   - Windows
   - MSRV
   - stable
   - `--no-default-features`
   - `--all-features`

3. 对 `public-api` 检查增加更新脚本和维护说明。

4. 增加手动 live workflow：
   - OpenAI
   - Zhipu
   - MiniMax
   - ZenMux

5. README / docs 收尾：
   - FAQ
   - migration notes
   - provider capability matrix
   - examples 索引页

6. 对照 `openai-node/examples` 再补一轮专题 examples，重点补：
   - `ui-generation`
   - `chat-params-types` 的 Rust 等价示例
   - Azure Realtime
   - 更多 stream-to-client 变体
   - parsing tools 系列变体

### 建议涉及文件

- [.github/workflows](/Users/yuyuehui/open-ai/openai-rs/.github/workflows)
- [scripts](/Users/yuyuehui/open-ai/openai-rs/scripts)
- [README.md](/Users/yuyuehui/open-ai/openai-rs/README.md)
- [README.zh.md](/Users/yuyuehui/open-ai/openai-rs/README.zh.md)
- [docs](/Users/yuyuehui/open-ai/openai-rs/docs)
- [examples](/Users/yuyuehui/open-ai/openai-rs/examples)

### 交付物

- release workflow
- 更完整的 CI 矩阵
- 收尾后的 README / docs / examples

### 验收标准

- 发布前检查可自动跑通
- 多平台与多 feature 组合有正式门禁
- README / docs / examples 的主题覆盖接近 `openai-node`

### 评分目标

- README / examples：`89 -> 94+`
- 工程化与发布成熟度：`71 -> 88+`

### 风险

- workflow 增多会提升 CI 时间和维护成本，需要区分快速路径与慢路径

## 11. 建议实施顺序

推荐严格按以下顺序推进：

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5

不能跳过的原因：

- 不先补测试，就不适合大规模强类型化和拆模块
- 不先补强运行时边界，就不适合继续扩复杂流式 API
- 不先收敛结构，就不适合把发布工程化做重

## 12. 每个 phase 的建议 PR 切分

为了保证可审查性，建议每个 phase 再切成多个小 PR。

### Phase 1 建议拆法

1. `retry_timeout + logger`
2. `path_query + uploads`
3. `parser + Azure`
4. `live workflow`

### Phase 2 建议拆法

1. `SSE parser + partial JSON`
2. `Responses stream runtime`
3. `Assistant stream`
4. `Realtime / Responses WebSocket`

### Phase 3 建议拆法

1. `images + audio`
2. `fine_tuning + batches`
3. `conversations + evals`
4. `containers + skills + videos`

### Phase 4 建议拆法

1. `resources` 结构拆分
2. `stream` 结构拆分
3. `websocket` 结构拆分
4. ADR / 模块文档

### Phase 5 建议拆法

1. release workflow
2. CI matrix
3. docs / README / examples 收尾

## 13. 最终目标状态

当 6 个 phase 全部完成后，目标状态应为：

- 功能覆盖保持 `95+`
- 主链路可用性提升到 `92+`
- 测试广度提升到 `85+`
- 测试深度提升到 `88+`
- 代码质量提升到 `88+`
- 可维护性提升到 `89+`
- 工程化与发布成熟度提升到 `88+`
- 公开库人体工学提升到 `89+`
- 综合评分从 `83` 提升到 **`90+`**

## 14. 一句话结论

`openai-rs` 下一阶段最正确的方向，不是“再补几个功能点”，而是：

- 先把测试和运行时边界补牢
- 再把长尾资源强类型化
- 再把结构和发布工程化做成长期可维护状态

按本文方案推进，`openai-rs` 有机会从“已经可用的社区 Rust SDK”，提升到“可以长期公开维护并稳定演进的成熟 SDK”。

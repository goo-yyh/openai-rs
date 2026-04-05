# openai-rs 长期公开库优化方案

## 1. 文档目标

本文档基于当前 `openai-rs` 的实现状态，给出一个面向“长期公开发布与持续维护”的优化路线。

方案目标不是继续堆功能，而是把当前已经可用的 SDK，逐步收敛为一个：

- 公开 API 稳定
- feature 边界清晰
- 文档与测试可持续
- 对外发布成本可控
- 后续演进具备 semver 纪律

的正式库。

## 2. 当前状态摘要

当前仓库已经具备：

- OpenAI / Azure / 多兼容 Provider
- HTTP / SSE / WebSocket 运行时
- `chat.completions`、`responses`、`models`、`files`、`uploads` 等高频路径
- 长尾资源命名空间完整暴露
- 单元测试、合约测试、WebSocket 集成测试、Provider live smoke tests

当前主要问题不在“能不能用”，而在“能不能长期稳定对外承诺”：

- 公开 API 面偏大，内部模块暴露较多
- 默认 feature 偏重
- 长尾资源强类型程度不足
- `resources/mod.rs` 体量过大
- 对 semver、MSRV、feature 组合、发布流程的约束还不够成体系

## 3. 总体原则

整个优化过程建议遵循以下原则：

### 3.1 先稳边界，再做人体工学

优先处理：

- 公开 API 面
- feature 设计
- 发布约束
- CI / 测试矩阵

然后再逐步处理：

- 长尾资源强类型
- 更细的人体工学 builder
- 更丰富的文档与示例

### 3.2 避免一次性重构过深

不要在同一个 phase 同时完成：

- 目录大拆分
- 全资源强类型化
- codegen 改造
- 公共 API 重新设计

这些动作耦合太高，风险大，回归成本高。

### 3.3 所有 phase 都要有“可发布状态”

每个 phase 结束后都应满足：

- `cargo build`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- README 和变更说明同步

## 4. 优化目标

本路线的最终目标分为五类：

1. API 稳定性
2. 代码结构可维护性
3. 类型安全与人体工学
4. 测试与兼容性保障
5. 发布、文档和生态成熟度

## 5. Phase 划分

建议分为 6 个 phase。

---

## Phase 0：发布基线与约束固化

### 目标

先把“这到底是一个怎样的公开库”定义清楚，避免后续每一步都在移动基础假设。

### 范围

- 固定并声明 MSRV
- 明确 semver 策略
- 修正 crate 元数据
- 明确 feature 策略
- 整理对外支持范围

### 具体工作

1. 修正 `Cargo.toml` 元数据

- `repository`
- `homepage`
- `documentation`
- `keywords`
- `categories`
- license / readme / description 再校准

2. 在 README 与 spec 中声明：

- 当前 MSRV
- semver 承诺
- breaking change 定义
- Provider 支持级别

3. 建立支持矩阵

- OpenAI：一等支持
- Azure：一等支持
- Zhipu / MiniMax / ZenMux：兼容支持
- Custom：接口稳定但行为由用户自行负责

4. 重新审视默认 feature

建议改为更轻的默认集合，例如：

- `stream`
- `multipart`
- `webhooks`
- `rustls-tls`

而把以下能力改为按需开启：

- `realtime`
- `responses-ws`
- `structured-output`
- `tool-runner`

### 交付物

- `Cargo.toml` 元数据整理完成
- README 增加版本策略、MSRV、feature 说明
- 新增发布策略文档或在 `0003_improve.md` 中固化

### 验收标准

- crate 元数据完整可用于公开发布
- 默认安装不会强制拉入 WebSocket 依赖
- README 可以清楚说明默认能力与可选能力

### 风险

- feature 默认值调整属于 API 使用体验变化，需要在 release notes 中显式说明

---

## Phase 1：公开 API 面收缩与稳定层建立

### 目标

把“内部实现细节”和“对外稳定接口”分离开，降低未来每次重构都触发 breaking change 的概率。

### 范围

- 收缩 `pub mod`
- 建立精确 re-export 层
- 区分稳定 API 与内部 API
- 明确不承诺稳定的模块边界

### 具体工作

1. 收缩根模块暴露面

当前不建议长期直接暴露的模块：

- `transport`
- `resource`
- 大量实现细节模块

建议改为：

- 内部模块保持 `mod`
- 只通过 `pub use` 导出正式稳定类型

2. 定义稳定 API 分层

建议将公开面分成三层：

- 顶层入口：`Client`、`ClientBuilder`、核心资源命名空间
- 稳定基础类型：错误、响应、分页、流、上传、Webhook
- 稳定 helper：structured output、tool runner

3. 对“不保证长期稳定”的 API 做收敛

例如：

- 过于内部的 builder 细节
- provider 内部 profile 结构
- transport 内部 request 拼装细节

4. 为未来 breaking change 做好过渡机制

- 引入 `#[deprecated]` 路径
- 建立迁移说明模板

### 交付物

- `lib.rs` 精简
- 稳定 `pub use` 清单
- 内部模块可见性调整

### 验收标准

- 对外暴露的模块和类型是有意设计的，而不是“因为文件存在所以直接暴露”
- 未来重构 transport / provider / resource internals 时，不需要轻易改动公共 API

### 风险

- 收缩 API 面可能引入 breaking change，需要作为主版本边界或在 `0.x` 阶段明确公告

---

## Phase 2：代码结构拆分与可维护性提升

### 目标

解决当前单文件过大、资源实现集中、审查困难的问题，为后续强类型化和 codegen 做准备。

### 范围

- 拆分 `resources/mod.rs`
- 拆分大模块
- 统一目录结构
- 降低文件体量与模块耦合

### 具体工作

1. 拆分资源目录

建议目标结构：

```text
src/resources/
├── mod.rs
├── chat.rs
├── responses.rs
├── models.rs
├── files.rs
├── uploads.rs
├── audio.rs
├── images.rs
├── vector_stores.rs
├── beta/
│   ├── mod.rs
│   ├── assistants.rs
│   ├── threads.rs
│   └── realtime.rs
└── common.rs
```

2. 拆分 builder 与数据类型

建议把：

- 数据结构
- 请求 builder
- 资源命名空间

分别拆开，避免“一个文件里同时有类型定义、业务逻辑、HTTP builder、子资源路由”。

3. 重构共享逻辑

提炼共用抽象，例如：

- list / bytes / json builder 公共层
- multipart helper
- beta header helper
- WebSocket 握手 helper

### 交付物

- 资源目录拆分完成
- 大型模块平均体量显著下降
- 内部依赖方向更清晰

### 验收标准

- 单文件体量明显下降
- 新资源或新 helper 的新增成本降低
- 代码审查可按命名空间进行

### 风险

- 这是一次机械性重构，容易引入路径和可见性问题，必须依赖稳定测试兜底

---

## Phase 3：类型系统强化与公开 API 人体工学优化

### 目标

在不破坏已有功能面的前提下，逐步把高价值长尾资源从 `Value` 提升为强类型 API。

### 范围

- 长尾资源强类型化
- WebSocket 事件强类型化
- Builder API 统一
- 错误与枚举语义增强

### 具体工作

1. 长尾资源按收益排序强类型化

建议优先顺序：

1. `beta.threads / runs / assistants`
2. `vector_stores`
3. `containers`
4. `skills`
5. `videos`

2. WebSocket 事件强类型化

当前 `RealtimeServerEvent` / `ResponsesServerEvent` 仍是通用 map 风格。

建议逐步提升为：

- 顶层事件 enum
- 关键事件对应结构体
- 未知事件保留 fallback 变体

即：

- 既保证强类型体验
- 又保留 forward compatibility

3. 统一 builder 行为

统一下列能力的命名和边界：

- `send`
- `send_with_meta`
- `send_raw`
- `stream`
- `ws().connect()`
- `parse`
- `run_tools`

4. 更细化错误类型

考虑补强：

- 参数缺失错误
- Provider 不支持能力错误
- WebSocket 协议错误 vs 业务错误

### 交付物

- 一批高频长尾资源强类型化
- WebSocket 事件类型升级
- builder 体验统一

### 验收标准

- 高频路径尽量减少直接手写 `serde_json::Value`
- IDE 自动补全体验明显提升
- WebSocket 事件消费时的类型断言更少

### 风险

- 强类型化容易导致 schema 演进压力，需要保留 `extra` 字段与未知枚举兜底

---

## Phase 4：测试矩阵、兼容性与回归保障体系

### 目标

把“当前测试能通过”升级为“长期演进下的行为边界可验证”。

### 范围

- feature matrix
- snapshot tests
- semver/API diff
- 文档示例编译
- provider 分级测试策略

### 具体工作

1. 加入 feature matrix CI

至少覆盖：

- default features
- `--no-default-features`
- `--features stream`
- `--features realtime`
- `--features responses-ws`
- `--all-features`

2. 增加 snapshot tests

适合做快照的对象：

- 关键请求序列化结果
- 错误映射结果
- SSE 聚合结果
- WebSocket 事件解码结果

3. 增加 semver/API 检查

建议在 CI 中加入公开 API diff 工具，避免无意暴露或删除公共符号。

4. 让 README 示例参与编译

将核心示例改造成：

- `examples/`
- 或 doc test compile targets

至少保证“文档不是假的”。

5. 明确 provider 测试分层

- 单元测试：纯规则校验
- 合约测试：mock server
- 集成测试：真实协议行为
- live tests：真实 provider，默认 ignore

### 交付物

- CI feature matrix
- snapshot tests 目录
- semver 检查
- examples 编译检查

### 验收标准

- 任意 feature 组合都能编译
- 关键序列化与聚合行为可以快照回归
- 公共 API 变化会被自动发现

### 风险

- CI 时间会上升，需要控制矩阵规模和并发策略

---

## Phase 5：文档、可观测性与发布工程化

### 目标

把仓库从“开发者可用”升级到“外部用户可理解、可接入、可排障、可持续发布”。

### 范围

- README 深化
- docs.rs 文档组织
- examples
- tracing/observability
- 发布自动化

### 具体工作

1. 文档体系化

建议至少补齐：

- 快速开始
- feature 说明
- Provider 兼容矩阵
- Azure 专题
- Realtime / Responses 流式专题
- Structured Output / Tool Runner 专题
- 迁移指南

2. 补 examples

建议加：

- `examples/openai_chat.rs`
- `examples/openai_responses.rs`
- `examples/azure_chat.rs`
- `examples/realtime_ws.rs`
- `examples/files_upload.rs`

3. 统一 tracing 字段

例如：

- request id
- provider
- endpoint id
- retry count
- websocket url / event type

4. 发布工程化

建议加入：

- release checklist
- changelog 规范
- tag 与 crates.io 发布流程
- docs.rs 构建检查

### 交付物

- 文档与 examples 成熟化
- tracing 输出规范
- 发布流程文档

### 验收标准

- 新用户可以通过 README + examples 完成接入
- 常见错误可以通过文档和日志定位
- 发布流程标准化，不依赖手工记忆

### 风险

- 文档和 examples 容易漂移，需要与 CI 绑定

---

## Phase 6：中长期演进与自动化生成

### 目标

在前面 5 个 phase 稳定后，再考虑更深的架构升级，降低长期维护成本。

### 范围

- codegen
- schema 驱动类型生成
- provider 扩展机制优化
- 性能与内存优化

### 具体工作

1. 评估是否引入 codegen

适合 codegen 的部分：

- 大量重复资源路由
- 请求 / 响应 schema
- 事件类型

不建议过早 codegen 的部分：

- tool runner
- structured output helper
- provider 兼容策略
- Rust 风格的人体工学 builder

2. 保持“生成代码 + 手写层”分离

建议分层：

- generated types / routes
- handwritten ergonomic facade

3. 评估性能热点

例如：

- SSE 聚合中的字符串拼接
- WebSocket 事件分发
- 大对象序列化与 clone

### 交付物

- codegen 可行性评估
- 原型或 ADR 文档

### 验收标准

- 引入 codegen 后不会破坏当前手写 API 的清晰性
- 维护成本下降，而不是把复杂度转移到生成链路

### 风险

- 过早 codegen 会放大复杂度，不应早于 Phase 3/4

## 6. 推荐执行顺序

推荐严格按以下顺序推进：

1. Phase 0
2. Phase 1
3. Phase 4
4. Phase 2
5. Phase 3
6. Phase 5
7. Phase 6

原因：

- Phase 0/1 决定发布边界
- Phase 4 提供重构安全网
- 有了边界和安全网，再做 Phase 2/3 的结构与类型升级
- Phase 5/6 放在后面，避免文档和自动化跟着反复返工

## 7. 优先级建议

如果只能先做一小批，我建议优先完成这四件事：

1. 收缩公开 API 面，稳定 `lib.rs`
2. 调整默认 feature，减轻安装与编译负担
3. 上 feature matrix 和 semver/API diff 检查
4. 拆分 `resources/mod.rs`

这四件事能最大幅度降低未来维护成本。

## 8. 里程碑定义

建议定义三个里程碑：

### Milestone A：可发布基线

完成：

- Phase 0
- Phase 1
- Phase 4 的基础部分

标志：

- 可以作为首个“正式公开推荐版本”发布

### Milestone B：公开库成熟版

完成：

- Phase 2
- Phase 3 的第一批高价值资源强类型化
- Phase 5 的核心文档与 examples

标志：

- API 稳定性和可用性达到公开库成熟水平

### Milestone C：长期维护版

完成：

- Phase 3 后续资源
- Phase 5 完整工程化
- Phase 6 可行性评估或原型

标志：

- 可以稳定承接长期迭代与外部用户增长

## 9. 结论

当前 `openai-rs` 已经完成“功能完整”的第一目标，下一阶段的核心不再是新增接口，而是：

- 控制公开 API 边界
- 建立 feature 和 semver 纪律
- 补齐测试矩阵与回归机制
- 渐进式强化类型系统
- 提升文档和发布成熟度

按本方案推进后，`openai-rs` 可以从“功能完成的 SDK”升级为“适合长期公开维护的 Rust 库”。

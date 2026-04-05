# ADR 0001: codegen 与手写层分离策略

## 状态

Accepted

## 背景

`openai-rs` 资源面较广，长尾命名空间存在大量重复的：

- 路由声明
- 请求 / 响应类型
- 列表与子资源模式

如果全部纯手写，长期维护成本会持续上升。

但如果过早全面 codegen，也会带来新的复杂度：

- 生成链路维护
- 人体工学 API 退化
- Provider 兼容逻辑难以内嵌

## 决策

采用“生成代码 + 手写外观层”分离策略：

- generated 层只负责 schema、路由和重复类型
- handwritten 层负责 builder、人机工程、provider 兼容、tool runner、structured output

## 不提前 codegen 的部分

- tool runner
- structured output helper
- provider 兼容策略
- Rust 风格 builder

## 进入 codegen 的前置条件

- 公开 API 边界稳定
- feature matrix 与 public API 检查已成熟
- `resources` 目录结构已经拆清

## 后续动作

- 先评估 `beta`、`vector_stores`、`containers` 的重复度
- 再决定是否引入 schema 驱动的局部 codegen

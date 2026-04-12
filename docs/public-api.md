# public-api 基线维护说明

`openai-core` 用 `cargo-public-api` 维护一份公开 API 基线，文件位于 `public-api/all-features.txt`。

目标不是阻止重构，而是把任何公开 API 变化都变成显式决策。

## 日常检查

本地或 CI 中使用：

```bash
bash ./scripts/check-public-api.sh
```

这个脚本会：

- 用 `--all-features` 生成当前公开 API
- 与 `public-api/all-features.txt` 做 `diff`
- 在 API 漂移时直接失败

如果本机没有安装 `cargo-public-api`，脚本会提示先执行：

```bash
cargo install cargo-public-api
```

## 何时更新基线

只有在以下场景才应该更新：

- 有意新增公开类型、方法、字段或 feature-gated 入口
- 有意删除或收缩公开 API，并且已经确认属于 semver 允许范围
- 公开 API 的命名、模块边界或 feature 暴露经过评审后确认要调整

不要因为“CI 红了”就直接更新基线。

## 更新方式

确认公开 API 变化是有意的之后，执行：

```bash
bash ./scripts/update-public-api.sh
```

然后在 PR 里同时说明：

- 哪些公开 API 发生了变化
- 这次变化是否影响 semver
- 是否需要补 `docs/migration.md`
- 是否需要在 `CHANGELOG.md` 记录

## 维护建议

- 默认以 `--all-features` 作为基线，避免 feature 组合导致的公开入口漏检
- 任何 breaking change 都要先更新迁移说明，再更新基线
- 如果变更只发生在实现层而非公开 API，基线文件不应变化

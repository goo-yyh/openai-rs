# 发布检查清单

phase 5 之后，发布前建议按“元数据 -> 构建 -> crates.io dry-run -> 发布后回查”的顺序执行。

## 1. 元数据检查

1. `Cargo.toml` 的 `version`、`documentation`、`repository` 是否正确
2. `CHANGELOG.md` 是否包含这次发布对应的变更说明
3. README、`docs/`、examples 是否与当前 API 同步
4. 如有公开 API 变化，是否同步更新了 `docs/migration.md`

可以先执行：

```bash
RELEASE_VERSION=0.1.0 bash ./scripts/check-release.sh
```

这个脚本会检查：

- `Cargo.toml` 里的版本号
- `CHANGELOG.md` 是否至少存在 `## Unreleased` 或对应版本标题
- docs.rs 文档地址字段是否存在

## 2. 本地验证

发布前至少跑一轮：

1. `cargo build`
2. `cargo test`
3. `cargo test --all-features`
4. `cargo check --no-default-features`
5. `cargo clippy --all-targets --all-features -- -D warnings`
6. `cargo deny check`
7. `cargo check --examples --all-features`
8. `bash ./scripts/check-ecosystem.sh`
9. `python3 ./scripts/generate_endpoints.py --check`
10. `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --no-deps`
11. `bash ./scripts/check-public-api.sh`

如果这次发布包含 provider 兼容性变更，建议额外跑手动 live workflow。

## 3. Release workflow

仓库提供了手动 `Release Readiness` workflow，建议在发布前至少跑一次，它会执行：

- 版本 / changelog 检查
- `cargo fmt --all -- --check`
- generated endpoint catalog 校验
- `cargo deny check`
- public API 基线校验
- `cargo test --no-default-features`
- `cargo test --all-features`
- examples / ecosystem fixtures 校验
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo publish --dry-run --all-features`
- docs.rs 风格文档构建检查

## 4. public-api 基线

如果 `bash ./scripts/check-public-api.sh` 失败，不要直接改基线文件。

先确认：

- 这是有意的公开 API 变化
- semver 影响已经评估
- `CHANGELOG.md` 和 `docs/migration.md` 已同步

确认后再执行：

```bash
bash ./scripts/update-public-api.sh
```

更多维护约定见 `docs/public-api.md`。

## 5. 发布顺序

1. 更新 `CHANGELOG.md`
2. 确认 public API 基线是否需要更新
3. 运行 `Release Readiness` workflow
4. `cargo publish`
5. 打 tag / GitHub Release
6. 检查 docs.rs 构建与 crates.io 页面

## 版本策略

- patch 不引入有意的 breaking change
- minor 可以收敛 API，但必须同步迁移说明与 public API 基线
- 任何 feature 暴露变化都应视为公开兼容性变更来审查

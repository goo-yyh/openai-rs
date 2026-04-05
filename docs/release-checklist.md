# 发布检查清单

发布前建议逐项确认：

1. `cargo build`
2. `cargo test`
3. `cargo test --all-features`
4. `cargo check --no-default-features`
5. `cargo clippy --all-targets --all-features -- -D warnings`
6. `cargo deny check`
7. `cargo check --examples --all-features`
8. `cargo doc --all-features --no-deps`
9. `bash ./scripts/check-public-api.sh`
10. README、`docs/`、`CHANGELOG.md` 是否同步

## 版本策略

- patch 不引入有意的 breaking change
- minor 可做公开 API 收敛，但必须同步迁移说明

## 发布顺序

1. 更新 `CHANGELOG.md`
2. 确认 public API 基线是否需要更新
3. 打 tag
4. 发布 crates.io
5. 检查 docs.rs 构建结果

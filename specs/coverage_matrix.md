# `openai-rs` Coverage Matrix

## Phase 1 Test Topics

| Topic | Status | Coverage |
| --- | --- | --- |
| `retry-after` | covered | `tests/retry_timeout.rs` |
| `retry-after-ms` | covered | `tests/retry_timeout.rs` |
| timeout retry | covered | `tests/retry_timeout.rs` |
| `OPENAI_LOG` | covered | `tests/logger.rs` |
| custom logger | covered | `tests/logger.rs`, `tests/retry_timeout.rs` |
| `send_with_meta()` | covered | `tests/retry_timeout.rs`, `tests/path_query.rs` |
| `send_raw()` | covered | `tests/path_query.rs` |
| path segment encoding | covered | `tests/path_query.rs` |
| query merge / encoding | covered | `tests/path_query.rs` |
| `to_file()` input paths | covered | `tests/files.rs`, `tests/uploads.rs` |
| multipart body + file mix | covered | `tests/uploads.rs` |
| streaming multipart upload | covered | `tests/uploads.rs` |
| structured output parse | covered | `tests/parser.rs` |
| finish reason parse errors | covered | `tests/parser.rs` |
| partial JSON runtime parsing | covered | `tests/parser.rs` |
| raw SSE event parsing | covered | `tests/parser.rs` |
| Azure endpoint | covered | `tests/azure.rs` |
| Azure deployment | covered | `tests/azure.rs` |
| Azure bearer auth | covered | `tests/azure.rs` |
| Azure realtime path | covered | `tests/azure.rs`, `tests/websocket.rs` |
| manual live workflow | covered | `.github/workflows/live.yml` |

## Contract Split

Moved out of the historical large contract tests into dedicated files:

- logger topics: `tests/logger.rs`
- path and query topics: `tests/path_query.rs`
- upload topics: `tests/uploads.rs`
- parser and stream topics: `tests/parser.rs`
- Azure topics: `tests/azure.rs`
- retry and timeout topics: `tests/retry_timeout.rs`

## Remaining Gaps

Still worth expanding later, but not required for Phase 1 completion:

- more `connection refused` retry cases
- deeper SSE framing edge cases
- more Azure realtime action endpoints beyond path coverage
- broader snapshot coverage for logger / retry / Azure error shapes

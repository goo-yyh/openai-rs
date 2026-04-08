#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES_DIR="${ROOT_DIR}/ecosystem-tests/fixtures"
SMOKE_SERVER="${ROOT_DIR}/scripts/ecosystem_smoke_server.py"

PORT="${OPENAI_RS_FIXTURE_PORT:-$(
python3 - <<'PY'
import socket

with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)}"

export OPENAI_RS_FIXTURE_BASE_URL="http://127.0.0.1:${PORT}/v1"

echo "==> starting ecosystem smoke server on ${OPENAI_RS_FIXTURE_BASE_URL}"
python3 "${SMOKE_SERVER}" --port "${PORT}" >/tmp/openai-rs-ecosystem-smoke.log 2>&1 &
SERVER_PID=$!
trap 'kill "${SERVER_PID}" >/dev/null 2>&1 || true' EXIT
sleep 1

find "${FIXTURES_DIR}" -mindepth 2 -maxdepth 2 -name Cargo.toml | sort | while read -r manifest; do
  echo "==> cargo check --offline --manifest-path ${manifest}"
  cargo check --offline --manifest-path "${manifest}"
  echo "==> cargo run --offline --manifest-path ${manifest}"
  cargo run --offline --manifest-path "${manifest}"
done

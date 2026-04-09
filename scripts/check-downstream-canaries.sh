#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANARIES_DIR="${ROOT_DIR}/downstream-canaries"

find "${CANARIES_DIR}" -mindepth 2 -maxdepth 2 -name Cargo.toml | sort | while read -r manifest; do
  echo "==> cargo check --offline --manifest-path ${manifest}"
  cargo check --offline --manifest-path "${manifest}"
  echo "==> cargo run --offline --manifest-path ${manifest}"
  cargo run --offline --manifest-path "${manifest}"
done

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${ROOT_DIR}/public-api"
OUTPUT_FILE="${OUTPUT_DIR}/all-features.txt"

mkdir -p "${OUTPUT_DIR}"

if ! cargo public-api --help >/dev/null 2>&1; then
  echo "cargo-public-api is required. Install it with: cargo install cargo-public-api"
  exit 1
fi

cargo public-api \
  --manifest-path "${ROOT_DIR}/Cargo.toml" \
  --all-features \
  --simplified \
  --color never > "${OUTPUT_FILE}"

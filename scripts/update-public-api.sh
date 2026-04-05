#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${ROOT_DIR}/public-api"
OUTPUT_FILE="${OUTPUT_DIR}/all-features.txt"

mkdir -p "${OUTPUT_DIR}"

cargo public-api \
  --manifest-path "${ROOT_DIR}/Cargo.toml" \
  --all-features \
  --simplified \
  --color never > "${OUTPUT_FILE}"

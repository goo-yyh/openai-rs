#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${ROOT_DIR}/public-api/all-features.txt"
TMP_FILE="$(mktemp)"
trap 'rm -f "${TMP_FILE}"' EXIT

cargo public-api \
  --manifest-path "${ROOT_DIR}/Cargo.toml" \
  --all-features \
  --simplified \
  --color never > "${TMP_FILE}"

diff -u "${BASELINE}" "${TMP_FILE}"

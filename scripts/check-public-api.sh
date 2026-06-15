#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${ROOT_DIR}/public-api/all-features.txt"
TMP_FILE="$(mktemp)"
trap 'rm -f "${TMP_FILE}"' EXIT
PUBLIC_API_TOOLCHAIN="${PUBLIC_API_TOOLCHAIN:-nightly-2026-06-08}"

if ! cargo +"${PUBLIC_API_TOOLCHAIN}" public-api --help >/dev/null 2>&1; then
  echo "cargo-public-api is required. Install it with: cargo install cargo-public-api"
  exit 1
fi

cargo +"${PUBLIC_API_TOOLCHAIN}" public-api \
  --manifest-path "${ROOT_DIR}/Cargo.toml" \
  --all-features \
  --simplified \
  --color never > "${TMP_FILE}"

diff -u "${BASELINE}" "${TMP_FILE}"

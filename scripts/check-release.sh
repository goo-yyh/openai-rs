#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="${ROOT_DIR}/Cargo.toml"
CHANGELOG="${ROOT_DIR}/CHANGELOG.md"
EXPECTED_VERSION="${RELEASE_VERSION:-}"

ACTUAL_VERSION="$(
  sed -n 's/^version = "\(.*\)"/\1/p' "${CARGO_TOML}" | head -n 1
)"

if [[ -z "${ACTUAL_VERSION}" ]]; then
  echo "failed to read package version from Cargo.toml"
  exit 1
fi

if [[ -n "${EXPECTED_VERSION}" && "${EXPECTED_VERSION}" != "${ACTUAL_VERSION}" ]]; then
  echo "release version mismatch: expected ${EXPECTED_VERSION}, found ${ACTUAL_VERSION}"
  exit 1
fi

if ! grep -Eq "^## (Unreleased|v?${ACTUAL_VERSION})$" "${CHANGELOG}"; then
  echo "CHANGELOG.md must contain either '## Unreleased' or a section for version ${ACTUAL_VERSION}"
  exit 1
fi

if ! grep -q '^documentation = "https://docs.rs/openai-rs"' "${CARGO_TOML}"; then
  echo "Cargo.toml is missing the docs.rs documentation field"
  exit 1
fi

echo "release metadata looks valid for version ${ACTUAL_VERSION}"

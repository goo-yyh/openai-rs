#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SNAPSHOT_DIR="${ROOT_DIR}/codegen/openapi"
SNAPSHOT_FILE="${SNAPSHOT_DIR}/openapi.documented.yml"
SNAPSHOT_JSON_FILE="${SNAPSHOT_DIR}/openapi.documented.json"
METADATA_FILE="${SNAPSHOT_DIR}/metadata.json"
DOCUMENTED_SPEC_URL="https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml"
OPENAPI_REPO_URL="https://github.com/openai/openai-openapi.git"

mkdir -p "${SNAPSHOT_DIR}"
curl -L --fail --max-time 120 "${DOCUMENTED_SPEC_URL}" -o "${SNAPSHOT_FILE}"

REPO_HEAD="$(git ls-remote "${OPENAPI_REPO_URL}" HEAD | awk '{print $1}')"
SPEC_VERSION="$(ruby -r yaml -e 'puts YAML.load_file(ARGV[0]).dig("info", "version")' "${SNAPSHOT_FILE}")"
ruby -r yaml -r json -e 'puts JSON.pretty_generate(YAML.load_file(ARGV[0]))' "${SNAPSHOT_FILE}" > "${SNAPSHOT_JSON_FILE}"

cat > "${METADATA_FILE}" <<EOF
{
  "documented_spec_url": "${DOCUMENTED_SPEC_URL}",
  "openai_openapi_repo_url": "${OPENAPI_REPO_URL}",
  "openai_openapi_repo_head": "${REPO_HEAD}",
  "documented_spec_version": "${SPEC_VERSION}",
  "fetched_at": "$(date +%F)"
}
EOF

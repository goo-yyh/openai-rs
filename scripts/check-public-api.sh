#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${ROOT_DIR}/public-api/all-features.txt"
TMP_FILE="$(mktemp)"
NORMALIZED_BASELINE="$(mktemp)"
NORMALIZED_TMP_FILE="$(mktemp)"
trap 'rm -f "${TMP_FILE}" "${NORMALIZED_BASELINE}" "${NORMALIZED_TMP_FILE}"' EXIT

normalize_public_api() {
  local input_file="$1"
  local output_file="$2"

  python3 - "${input_file}" "${output_file}" <<'PY'
import re
import sys


def split_top_level(params: str) -> list[str]:
    parts: list[str] = []
    current: list[str] = []
    paren = bracket = brace = angle = 0

    for ch in params:
        if ch == "," and paren == bracket == brace == angle == 0:
            parts.append("".join(current).strip())
            current = []
            continue

        if ch == "(":
            paren += 1
        elif ch == ")" and paren > 0:
            paren -= 1
        elif ch == "[":
            bracket += 1
        elif ch == "]" and bracket > 0:
            bracket -= 1
        elif ch == "{":
            brace += 1
        elif ch == "}" and brace > 0:
            brace -= 1
        elif ch == "<":
            angle += 1
        elif ch == ">" and angle > 0:
            angle -= 1

        current.append(ch)

    parts.append("".join(current).strip())
    return parts


def find_top_level_colon(param: str) -> int:
    paren = bracket = brace = angle = 0

    for index, ch in enumerate(param):
        if (
            ch == ":"
            and paren == bracket == brace == angle == 0
            and (index == 0 or param[index - 1] != ":")
            and (index + 1 == len(param) or param[index + 1] != ":")
        ):
            return index

        if ch == "(":
            paren += 1
        elif ch == ")" and paren > 0:
            paren -= 1
        elif ch == "[":
            bracket += 1
        elif ch == "]" and bracket > 0:
            bracket -= 1
        elif ch == "{":
            brace += 1
        elif ch == "}" and brace > 0:
            brace -= 1
        elif ch == "<":
            angle += 1
        elif ch == ">" and angle > 0:
            angle -= 1

    return -1


def normalize_param(param: str) -> str:
    if not param:
        return param

    if param in {"self", "&self", "&mut self"}:
        return param

    colon_index = find_top_level_colon(param)
    if colon_index < 0:
        return param

    name = param[:colon_index].strip()
    if re.fullmatch(r"(?:mut\s+)?[A-Za-z_][A-Za-z0-9_]*", name):
        return param[colon_index + 1 :].strip()

    return param


def normalize_signature(line: str) -> str:
    if not (line.startswith("pub fn ") or line.startswith("pub async fn ")):
        return line

    start = -1
    paren = bracket = brace = angle = 0
    for index, ch in enumerate(line):
        if ch == "(" and paren == bracket == brace == angle == 0:
            start = index
            break

        if ch == "(":
            paren += 1
        elif ch == ")" and paren > 0:
            paren -= 1
        elif ch == "[":
            bracket += 1
        elif ch == "]" and bracket > 0:
            bracket -= 1
        elif ch == "{":
            brace += 1
        elif ch == "}" and brace > 0:
            brace -= 1
        elif ch == "<":
            angle += 1
        elif ch == ">" and angle > 0:
            angle -= 1

    if start < 0:
        return line

    depth = 0
    end = -1
    for index, ch in enumerate(line[start:], start=start):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                end = index
                break

    if end < 0:
        return line

    params = line[start + 1 : end]
    normalized = ", ".join(normalize_param(part) for part in split_top_level(params))
    return f"{line[:start + 1]}{normalized}{line[end:]}"


input_path, output_path = sys.argv[1], sys.argv[2]
with open(input_path, encoding="utf-8") as input_file:
    lines = input_file.readlines()

with open(output_path, "w", encoding="utf-8") as output_file:
    for line in lines:
        output_file.write(normalize_signature(line))
PY
}

if ! cargo public-api --help >/dev/null 2>&1; then
  echo "cargo-public-api is required. Install it with: cargo install cargo-public-api"
  exit 1
fi

cargo public-api \
  --manifest-path "${ROOT_DIR}/Cargo.toml" \
  --all-features \
  --simplified \
  --color never > "${TMP_FILE}"

normalize_public_api "${BASELINE}" "${NORMALIZED_BASELINE}"
normalize_public_api "${TMP_FILE}" "${NORMALIZED_TMP_FILE}"

diff -u "${NORMALIZED_BASELINE}" "${NORMALIZED_TMP_FILE}"

#!/usr/bin/env bash

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
prepared="$here/.prepared"
mkdir -p "$prepared/cache"

nix_out() {
  nix build --no-link --json "nixpkgs#$1" | jq -r '.[0].outputs.out'
}

bash_path=$(nix_out bashInteractive)
coreutils_path=$(nix_out coreutils)
strace_path=$(nix_out strace)
uv_path=$(nix_out uv)
python_path=$(nix_out python312)
tool_path="$bash_path/bin:$coreutils_path/bin:$strace_path/bin:$uv_path/bin:$python_path/bin"

(
  cd "$here"
  PATH="$tool_path" UV_CACHE_DIR="$prepared/cache" UV_PYTHON="$python_path/bin/python" \
    UV_PYTHON_DOWNLOADS=never uv lock
  PATH="$tool_path" UV_CACHE_DIR="$prepared/cache" UV_PYTHON="$python_path/bin/python" \
    UV_PYTHON_DOWNLOADS=never UV_PROJECT_ENVIRONMENT="$prepared/prefetch-venv" \
    uv sync --locked
)

jq -n \
  --arg bash "$bash_path" \
  --arg coreutils "$coreutils_path" \
  --arg strace "$strace_path" \
  --arg uv "$uv_path" \
  --arg python "$python_path" \
  '{bash: $bash, coreutils: $coreutils, strace: $strace, uv: $uv, python: $python}' \
  >"$prepared/tools.json"

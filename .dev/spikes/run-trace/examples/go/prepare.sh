#!/usr/bin/env bash

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
prepared="$here/.prepared"
mkdir -p "$prepared/gomodcache"

nix_out() {
  nix build --no-link --json "nixpkgs#$1" | jq -r '.[0].outputs.out'
}

bash_path=$(nix_out bashInteractive)
coreutils_path=$(nix_out coreutils)
strace_path=$(nix_out strace)
go_path=$(nix_out go)
tool_path="$bash_path/bin:$coreutils_path/bin:$strace_path/bin:$go_path/bin"

(
  cd "$here"
  PATH="$tool_path" GOMODCACHE="$prepared/gomodcache" go mod tidy
  PATH="$tool_path" GOMODCACHE="$prepared/gomodcache" go mod download
  PATH="$tool_path" GOMODCACHE="$prepared/gomodcache" go mod vendor -o "$prepared/vendor"
)

jq -n \
  --arg bash "$bash_path" \
  --arg coreutils "$coreutils_path" \
  --arg strace "$strace_path" \
  --arg go "$go_path" \
  '{bash: $bash, coreutils: $coreutils, strace: $strace, go: $go}' \
  >"$prepared/tools.json"

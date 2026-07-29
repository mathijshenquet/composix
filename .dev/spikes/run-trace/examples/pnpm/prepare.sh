#!/usr/bin/env bash

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
prepared="$here/.prepared"
mkdir -p "$prepared/store"

nix_out() {
  nix build --no-link --json "nixpkgs#$1" | jq -r '.[0].outputs.out'
}

bash_path=$(nix_out bashInteractive)
coreutils_path=$(nix_out coreutils)
strace_path=$(nix_out strace)
node_path=$(nix_out nodejs)
pnpm_path=$(nix_out pnpm)
sed_path=$(nix_out gnused)
tool_path="$bash_path/bin:$coreutils_path/bin:$strace_path/bin:$node_path/bin:$pnpm_path/bin:$sed_path/bin"

(
  cd "$here"
  PATH="$tool_path" pnpm install --lockfile-only
  PATH="$tool_path" pnpm fetch --frozen-lockfile --store-dir "$prepared/store"
)

jq -n \
  --arg bash "$bash_path" \
  --arg coreutils "$coreutils_path" \
  --arg strace "$strace_path" \
  --arg node "$node_path" \
  --arg pnpm "$pnpm_path" \
  --arg sed "$sed_path" \
  '{bash: $bash, coreutils: $coreutils, strace: $strace, node: $node, pnpm: $pnpm,
    sed: $sed}' \
  >"$prepared/tools.json"

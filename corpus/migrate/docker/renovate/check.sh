#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
cix="${1:-$root/../../../../target/debug/cix}"
state_dir="$(mktemp -d)"
trap 'rm -rf -- "$state_dir"' EXIT

CIX_STATE_DIR="$state_dir" "$cix" build -t regrade "$root"
systemd-analyze calendar daily >/dev/null
CIX_STATE_DIR="$state_dir" "$cix" compose check "$root/compose.json"

printf 'PASS cix build, calendar, and compose validation\n'

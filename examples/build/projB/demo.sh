#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cix_bin=$(realpath "$(command -v "${CIX_BIN:-cix}")")
unit=

cleanup() {
  if [[ -n "$unit" ]]; then
    sudo systemctl stop "$unit" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

store_path=$("$cix_bin" build "$example_dir")
unit=$(sudo "$cix_bin" run "$store_path" --detach)
echo "started $unit"

for _ in {1..100}; do
  if page=$(curl --fail --silent http://127.0.0.1:18082/ 2>/dev/null); then
    break
  fi
  sleep 0.1
done
grep -Fx "hello from RUN v0" <<<"${page:-}"

sudo systemctl stop "$unit"
echo "stopped $unit"
unit=

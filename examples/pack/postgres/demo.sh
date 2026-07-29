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
postgres_bin=$(sed -n 's/.*"default": "\([^"]*\)".*/\1/p' "$store_path/cix-manifest.json" | cut -d: -f1)
[[ -n "$postgres_bin" ]]
unit=$(sudo "$cix_bin" run "$store_path" --detach)
echo "started $unit"

for _ in {1..100}; do
  if query=$(
    "$postgres_bin/psql" \
      --host=127.0.0.1 \
      --port=5432 \
      --username=cix \
      --dbname=postgres \
      --no-password \
      --tuples-only \
      --no-align \
      --command='SELECT 1' 2>/dev/null
  ); then
    echo "$query"
    break
  fi
  sleep 0.1
done
[[ ${query:-} == "1" ]]

"$cix_bin" ps
sudo systemctl stop "$unit"
echo "stopped $unit"
unit=

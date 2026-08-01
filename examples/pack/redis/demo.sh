#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cix_bin=$(realpath "${CIX_BIN:-$(command -v cix)}")
unit=

cleanup() {
  if [[ -n "$unit" ]]; then
    sudo systemctl stop "$unit" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

store_path=$(nix-build "$example_dir" --no-out-link)
redis_server=$(sed -n 's/.*"start":[[:space:]]*\["\([^"]*\)".*/\1/p' "$store_path/cix-manifest.json")
redis_bin=$(dirname "$redis_server")
[[ -n "$redis_bin" ]]
unit=$(sudo "$cix_bin" run "$store_path" --detach)
echo "started $unit"

for _ in {1..100}; do
  if tcp=$(
    "$redis_bin/redis-cli" -h 127.0.0.1 -p 6379 PING 2>/dev/null
  ) && socket=$(sudo "$redis_bin/redis-cli" -s /run/redis/redis.sock PING 2>/dev/null); then
    break
  fi
  sleep 0.1
done
[[ ${tcp:-} == PONG ]]
[[ ${socket:-} == PONG ]]
echo "$tcp over TCP; $socket over the Unix socket"

"$cix_bin" ps
sudo systemctl stop "$unit"
echo "stopped $unit"
unit=

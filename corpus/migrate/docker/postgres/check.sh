#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} != cix ]]; then
  echo "usage: $0 cix" >&2
  exit 2
fi

root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../../target/debug/cix"}
item=$(timeout 240 "$cix" build "$root#postgres")
unit=$(timeout 30 sudo -n "$cix" run --detach "$item")

cleanup() {
  sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT HUP INT TERM

timeout 60 "$cix" probe await tcp tcp://127.0.0.1:5432
status=$(timeout 15 sudo -n "$cix" exec "$unit" -- pg_isready -h 127.0.0.1 -p 5432)
[[ $status == *"accepting connections"* ]]
printf 'PASS pg_isready=%s\n' "$status"

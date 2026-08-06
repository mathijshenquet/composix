#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} != cix ]]; then
  echo "usage: $0 cix" >&2
  exit 2
fi

root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../../target/debug/cix"}
item=$(timeout 240 "$cix" build "$root#registry")
unit=$(timeout 30 sudo -n "$cix" run --detach "$item")

cleanup() {
  sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT HUP INT TERM

timeout 60 "$cix" probe await http http://127.0.0.1:5000/v2/
response=$(curl --fail --silent --show-error --max-time 5 http://127.0.0.1:5000/v2/)
[[ $response == "{}" ]]
printf 'PASS registry=%s\n' "$response"

#!/usr/bin/env bash
set -euo pipefail

if [[ ${1:-} != cix ]]; then
  echo "usage: $0 cix" >&2
  exit 2
fi

cix=${CIX:-../../../../target/debug/cix}
item=$($cix build .#mailpit)
unit=$($cix run --user --detach "$item")
cleanup() {
  systemctl --user stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT HUP INT TERM

timeout 15s "$cix" probe await http 127.0.0.1:8025/livez
curl --fail --silent --show-error --max-time 5 http://127.0.0.1:8025/livez >/dev/null

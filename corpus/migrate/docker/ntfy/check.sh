#!/usr/bin/env bash
set -euo pipefail

case ${1:-} in
  cix) ;;
  *)
    echo "usage: $0 cix" >&2
    exit 2
    ;;
esac

root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../../target/debug/cix"}
item=$(timeout 240 "$cix" build "$root#ntfy")
printf 'cix item %s\n' "$item"
unit=$(timeout 30 sudo -n "$cix" run --detach "$item")
[[ $unit == cix-run-ntfy-* ]]
printf 'cix unit %s\n' "$unit"

cleanup() {
  sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT HUP INT TERM

timeout 30 "$cix" probe await http http://127.0.0.1:80/v1/health
health=$(curl --fail --silent --show-error --max-time 5 http://127.0.0.1:80/v1/health)
[[ $health == '{"healthy":true}' ]]
printf 'PASS cix health=%s\n' "$health"

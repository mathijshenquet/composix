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
repo=$(cd -- "$root/../../../.." && pwd)
runtime_helper=$(nix build --no-link --print-out-paths "$repo#cix")/bin/cix
item=$(timeout 240 "$cix" build "$root#filebrowser")
printf 'cix item %s\n' "$item"
unit=$(timeout 30 sudo -n env "CIX_RUNTIME_HELPER=$runtime_helper" "$cix" run --detach "$item")
[[ $unit == cix-run-filebrowser-* ]]
printf 'cix unit %s\n' "$unit"

cleanup() {
  sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT HUP INT TERM

timeout 30 "$cix" probe await http http://127.0.0.1:80/health
health=$(curl --fail --silent --show-error --max-time 5 http://127.0.0.1:80/health)
[[ -n $health ]]
printf 'PASS cix health=%s\n' "$health"

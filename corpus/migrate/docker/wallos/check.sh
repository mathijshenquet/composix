#!/usr/bin/env bash
set -euo pipefail

mode=${1:?usage: ./check.sh docker|cix}
root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../../target/debug/cix"}
repo=$(cd -- "$root/../../../.." && pwd)
runtime_helper=$(nix build --no-link --print-out-paths "$repo#cix")/bin/cix
name=migrate-r5-wallos
container=
unit=

cleanup() {
  [[ -z $unit ]] || sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
  [[ -z $container ]] || docker rm -f "$container" >/dev/null 2>&1 || true
}
trap cleanup EXIT

probe() {
  for _ in $(seq 1 60); do
    if [[ $(curl --fail --silent --max-time 2 http://127.0.0.1:18092/health.php 2>/dev/null || true) == OK ]]; then
      return 0
    fi
    sleep 1
  done
  return 1
}

[[ -d $root/context ]] || { echo 'context/ missing — run ../../fetch.sh wallos first' >&2; exit 1; }
cd "$root"
case $mode in
  docker)
    image=$(timeout 1200 docker build --quiet --file Dockerfile --tag "$name" context)
    printf 'docker image %s\n' "$image"
    container=$(timeout 30 docker run --detach --rm --publish 127.0.0.1:18092:80 "$name")
    probe
    printf 'PASS docker\n'
    ;;
  cix)
    item=$(timeout 1200 "$cix" build "$root#wallos")
    printf 'cix item %s\n' "$item"
    unit=$(timeout 30 sudo -n env CIX_RUNTIME_HELPER="$runtime_helper" "$cix" run --detach "$item" | tail -n1)
    printf 'cix unit %s\n' "$unit"
    unit_text=$(sudo -n systemctl cat "$unit")
    [[ $unit_text == *"\"$runtime_helper\" \"probe\""* ]]
    [[ $unit_text != *target/debug/cix* ]]
    probe
    printf 'PASS cix\n'
    ;;
  *) echo 'usage: ./check.sh docker|cix' >&2; exit 1 ;;
esac

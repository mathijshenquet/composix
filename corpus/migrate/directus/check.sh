#!/usr/bin/env bash
set -euo pipefail
mode=${1:?usage: ./check.sh docker|cix}; root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../target/debug/cix"}; name=migrate-r5-directus; container=; unit=
cleanup() { [[ -z $unit ]] || sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true; [[ -z $container ]] || docker rm -f "$container" >/dev/null 2>&1 || true; }
trap cleanup EXIT
probe() { for _ in $(seq 1 90); do [[ $(curl --fail --silent --max-time 2 http://127.0.0.1:18093/server/ping 2>/dev/null || true) == pong ]] && return 0; sleep 1; done; return 1; }
[[ -d $root/context ]] || { echo 'context/ missing — run ../fetch.sh directus first' >&2; exit 1; }
cd "$root"
case $mode in
  docker) image=$(timeout 1200 docker build --quiet --file Dockerfile --tag "$name" context); printf 'docker image %s\n' "$image"; container=$(docker run --detach --rm --publish 127.0.0.1:18093:8055 "$name"); probe; printf 'PASS docker\n' ;;
  cix) item=$(timeout 1200 "$cix" build .#directus); printf 'cix item %s\n' "$item"; unit=$(sudo -n "$cix" run --detach "$item" | tail -n1); printf 'cix unit %s\n' "$unit"; probe; printf 'PASS cix\n' ;;
  *) echo 'usage: ./check.sh docker|cix' >&2; exit 1 ;;
esac

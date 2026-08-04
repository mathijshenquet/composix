#!/usr/bin/env bash
set -euo pipefail
mode=${1:?usage: ./check.sh docker|cix}; CIX=${CIX:-../../../../target/debug/cix}; name=migrate-r4-verdaccio; unit=
root=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ ! -d $root/context ]]; then echo 'context/ missing — run ../../fetch.sh verdaccio first' >&2; exit 1; fi
cleanup() { [ -z "$unit" ] || sudo systemctl stop "$unit" || true; docker rm -f "$name" >/dev/null 2>&1 || true; }; trap cleanup EXIT
if [ "$mode" = docker ]; then docker build --quiet -t "$name" -f Dockerfile context; docker run -d --rm --name "$name" -p 4873:4873 "$name" >/dev/null; else item=$($CIX build .#verdaccio); printf 'cix item %s\n' "$item"; unit=$(sudo "$CIX" run --detach "$item" | tail -n1); fi
for _ in {1..20}; do curl --max-time 2 --fail --silent http://127.0.0.1:4873/-/ping >/dev/null && exit 0; sleep 1; done
exit 1

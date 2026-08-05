#!/usr/bin/env bash
set -euo pipefail
mode=${1:?usage: ./check.sh docker|cix}; CIX=${CIX:-../../../../target/debug/cix}; name=migrate-r4-watchtower; unit=
root=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ ! -d $root/context ]]; then echo 'context/ missing — run ../../fetch.sh watchtower first' >&2; exit 1; fi
cleanup() { [ -z "$unit" ] || sudo systemctl stop "$unit" || true; docker rm -f "$name" >/dev/null 2>&1 || true; }; trap cleanup EXIT
if [ "$mode" = docker ]; then docker build --quiet -t "$name" -f Dockerfile context; docker run --rm --name "$name" -v /var/run/docker.sock:/var/run/docker.sock "$name" --run-once; else item=$($CIX build .#watchtower); printf 'cix item %s\n' "$item"; "$CIX" build --cold .#watchtower >/dev/null; "$CIX" inspect "$item" | rg -F '"/var/run/docker.sock"'; fi

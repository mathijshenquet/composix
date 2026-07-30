#!/usr/bin/env bash
set -euo pipefail
mode=${1:?usage: ./check.sh docker|cix}; CIX=${CIX:-../../../target/debug/cix}; name=migrate-r4-watchtower; unit=
cleanup() { [ -z "$unit" ] || sudo systemctl stop "$unit" || true; docker rm -f "$name" >/dev/null 2>&1 || true; }; trap cleanup EXIT
if [ "$mode" = docker ]; then docker build --quiet -t "$name" -f Dockerfile context; docker run --rm --name "$name" -v /var/run/docker.sock:/var/run/docker.sock "$name" --run-once; else item=$($CIX build . | tail -n1); unit=$(sudo "$CIX" run --detach "$item" | tail -n1); sleep 2; sudo systemctl is-active --quiet "$unit"; fi

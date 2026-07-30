#!/usr/bin/env bash
set -euo pipefail
mode=${1:?usage: ./check.sh docker|cix}; CIX=${CIX:-../../../target/debug/cix}; name=migrate-r4-redis; unit=
cleanup() { [ -z "$unit" ] || sudo systemctl stop "$unit" || true; docker rm -f "$name" >/dev/null 2>&1 || true; }; trap cleanup EXIT
if [ "$mode" = docker ]; then docker build --quiet -t "$name" -f Dockerfile context/7.4/alpine; docker run -d --rm --name "$name" -p 6379:6379 "$name" >/dev/null; else item=$($CIX build . | tail -n1); unit=$(sudo "$CIX" run --detach "$item" | tail -n1); fi
for _ in {1..20}; do timeout 2 bash -c 'exec 3<>/dev/tcp/127.0.0.1/6379; printf "PING\r\n" >&3; read -r reply <&3; case "$reply" in *PONG*) exit 0;; *) exit 1;; esac' && exit 0; sleep 1; done
exit 1

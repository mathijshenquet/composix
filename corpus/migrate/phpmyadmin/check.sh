#!/usr/bin/env bash
set -euo pipefail
mode=${1:?usage: ./check.sh docker|cix}; CIX=${CIX:-../../../target/debug/cix}; name=migrate-r4-phpmyadmin; unit=
cleanup() { [ -z "$unit" ] || sudo systemctl stop "$unit" || true; docker rm -f "$name" >/dev/null 2>&1 || true; }; trap cleanup EXIT
if [ "$mode" = docker ]; then docker build --quiet -t "$name" -f Dockerfile context; docker run -d --rm --name "$name" -p 8080:80 "$name" >/dev/null; else item=$($CIX build . | tail -n1); unit=$(sudo "$CIX" run --detach "$item" | tail -n1); fi
for _ in {1..20}; do curl --max-time 2 --fail --silent http://127.0.0.1:8080/ | grep -qi phpmyadmin && exit 0; sleep 1; done
exit 1

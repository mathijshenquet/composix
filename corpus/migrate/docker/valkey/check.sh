#!/bin/sh
set -eu

mode=${1-}
if [ "$mode" != cix ]; then
  echo "usage: $0 cix" >&2
  exit 2
fi

cix=${CIX:-../../../../target/debug/cix}

unit=
cleanup() {
  if [ -n "$unit" ]; then
    timeout 10s sudo systemctl stop "$unit" >/dev/null 2>&1 || sudo systemctl kill --kill-who=all --signal=SIGKILL "$unit" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

item=$($cix build .#valkey)
unit=$(timeout 30s sudo "$cix" run --detach "$item")
timeout 30s sudo "$cix" probe await tcp tcp://127.0.0.1:6379
response=$(timeout 10s sudo "$cix" exec "$unit" -- valkey-cli PING)
if [ "$response" != PONG ]; then
  echo "unexpected valkey-cli PING response: $response" >&2
  exit 1
fi
echo "$response"

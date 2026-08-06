#!/usr/bin/env bash
set -eu

case "${1:-}" in
  cix)
    cix=${CIX:-../../../../target/debug/cix}
    build_output=$(mktemp)
    unit=
    cleanup() {
      if test -n "$unit"; then
        sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
      fi
      rm -f "$build_output"
    }
    trap cleanup EXIT

    if test -n "${CIX_ITEM:-}"; then
      item=$CIX_ITEM
    else
      timeout 300 setsid --wait "$cix" build . >"$build_output"
      item=$(jq -er '.web' "$build_output")
    fi
    unit=$(sudo -n "$cix" run --detach "$item")
    status=$(curl --fail --silent --show-error --retry 10 --retry-connrefused --max-time 10 -o /dev/null -w '%{http_code}' http://127.0.0.1/)
    printf 'HTTP GET / -> %s\n' "$status"
    test "$status" = 200
    fallback_status=$(curl --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' http://127.0.0.1/not-a-real-route)
    printf 'HTTP GET /not-a-real-route -> %s\n' "$fallback_status"
    ;;
  *)
    echo "usage: $0 cix" >&2
    exit 2
    ;;
esac

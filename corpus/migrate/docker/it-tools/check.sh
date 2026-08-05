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

    timeout 300 setsid --wait "$cix" build . >"$build_output"
    item=$(jq -er '.web' "$build_output")
    unit=$(sudo -n "$cix" run --detach "$item")
    curl --fail --silent --show-error --retry 10 --retry-connrefused --max-time 10 http://127.0.0.1/ >/dev/null
    test "$(curl --fail --silent --show-error --max-time 10 http://127.0.0.1/)" = "$(curl --fail --silent --show-error --max-time 10 http://127.0.0.1/not-a-real-route)"
    ;;
  *)
    echo "usage: $0 cix" >&2
    exit 2
    ;;
esac

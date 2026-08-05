#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != cix ]]; then
  printf 'usage: %s cix\n' "$0" >&2
  exit 2
fi

cix=${CIX:-../../../../target/debug/cix}
item=$($cix build .#haproxy)
version=$("$item/bin/haproxy" -v)
grep -F 'HAProxy version 3.2.22' <<<"$version" >/dev/null

#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
  cix) ;;
  *)
    echo "usage: $0 cix" >&2
    exit 2
    ;;
esac

root_dir=$(cd "$(dirname "$0")" && pwd)
bin=${CIX:-"$root_dir/../../../../target/debug/cix"}
item=$($bin build "$root_dir"#broker)
unit=$(sudo "$bin" run --detach "$item")

cleanup() {
  sudo systemctl stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT

timeout 30s sudo "$bin" exec "$unit" -- bash -c '
  set -euo pipefail
  export LD_LIBRARY_PATH=/usr/lib:/lib
  rm -f /tmp/cix-mosquitto-message
  timeout 10s /usr/bin/mosquitto_sub -h 127.0.0.1 -p 1883 -t cix/roundtrip -C 1 > /tmp/cix-mosquitto-message &
  subscriber=$!
  trap "kill \"$subscriber\" 2>/dev/null || true" EXIT
  sleep 1
  /usr/bin/mosquitto_pub -h 127.0.0.1 -p 1883 -t cix/roundtrip -m cix-ok
  wait "$subscriber"
  IFS= read -r message < /tmp/cix-mosquitto-message
  [[ "$message" == cix-ok ]]
'

#!/bin/sh
set -eu

if [ "${1:-}" != cix ]; then
    echo "usage: $0 cix" >&2
    exit 2
fi

cix=${CIX:-../../../../target/debug/cix}
item=$($cix build .#homer)
unit=$(sudo -n "$cix" run --detach "$item")

cleanup() {
    sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

url="http://127.0.0.1:8080/"
i=0
while [ "$i" -lt 30 ]; do
    if body=$(curl --fail --silent --show-error --max-time 2 "$url"); then
        printf '%s' "$body" | grep -F '<div id="app-mount"></div>' >/dev/null
        exit 0
    fi
    i=$((i + 1))
    sleep 1
done

echo "Homer did not serve its root page at $url" >&2
exit 1

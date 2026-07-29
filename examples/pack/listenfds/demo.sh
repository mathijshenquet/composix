#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cix_bin=$(realpath "$(command -v "${CIX_BIN:-cix}")")
address=127.0.0.1:18081
unit=
socket=

cleanup() {
  if [[ -n "$unit" ]]; then
    sudo systemctl stop "$unit" >/dev/null 2>&1 || true
  fi
  if [[ -n "$socket" ]]; then
    sudo systemctl stop "$socket" >/dev/null 2>&1 || true
    sudo rm "/run/systemd/system/$socket" >/dev/null 2>&1 || true
  fi
  if [[ -n "$unit" ]]; then
    sudo rm "/run/systemd/system/$unit" >/dev/null 2>&1 || true
  fi
  sudo systemctl daemon-reload >/dev/null 2>&1 || true
}
trap cleanup EXIT

sudo -n true
if ss -ltn '( sport = :18081 )' | grep -q "$address"; then
  echo "$address is already in use" >&2
  exit 1
fi

store_path=$("$cix_bin" build "$example_dir")
unit=$(sudo "$cix_bin" run "$store_path" -p http="$address" --detach)
socket=${unit%.service}-http.socket
echo "started $unit and $socket"

for _ in {1..50}; do
  if page=$(curl --fail --silent --show-error "http://$address/" 2>/dev/null); then
    break
  fi
  sleep 0.1
done
[[ ${page:-} == "LISTEN_FDS=1; no socket() authority" ]]
echo "$page"

sudo systemctl show "$unit" \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=CapabilityBoundingSet \
  --property=SocketBindDeny
[[ $(sudo systemctl show "$unit" --property=PrivateNetwork --value) == yes ]]
[[ $(sudo systemctl show "$unit" --property=RestrictAddressFamilies --value) == AF_UNIX ]]
[[ -z $(sudo systemctl show "$unit" --property=CapabilityBoundingSet --value) ]]
[[ $(sudo systemctl show "$unit" --property=SocketBindDeny --value) == any ]]
sudo systemctl cat "$socket" | grep -Fx "ListenStream=$address"
sudo systemctl cat "$socket" | grep -Fx "FileDescriptorName=http"
sudo systemctl cat "$socket" | grep -Fx "Service=$unit"
"$cix_bin" ps | grep -F "$socket"

sudo systemctl stop "$unit"
! sudo systemctl is-active --quiet "$socket"
! ss -ltn '( sport = :18081 )' | grep -q "$address"
echo "stopped cleanly; service and listening socket are inactive"

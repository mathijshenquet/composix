#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
source "$example_dir/../lib/demo-lib.sh"

activation_base=cix-dstyle-listenfds
activation_service=$activation_base.service
activation_socket=$activation_base.socket
listen_address=127.0.0.1:18081

cleanup() {
  stop_unit "$activation_socket"
  stop_unit "$activation_service"
  if ss -ltn '( sport = :18081 )' | grep -q 127.0.0.1:18081; then
    echo "cleanup failed: activation socket still listens on $listen_address" >&2
    return 1
  fi
}
trap cleanup EXIT

sudo -n true
stop_unit "$activation_socket"
stop_unit "$activation_service"
if ss -ltn '( sport = :18081 )' | grep -q 127.0.0.1:18081; then
  echo "$listen_address is already in use" >&2
  exit 1
fi

store_path=$(nix-build "$example_dir" --no-out-link)
sudo systemd-run \
  --unit="$activation_base" \
  --collect \
  --service-type=exec \
  --socket-property=ListenStream="$listen_address" \
  --property=DynamicUser=yes \
  --property=PrivateNetwork=yes \
  --property=RestrictAddressFamilies=AF_UNIX \
  --property=CapabilityBoundingSet= \
  --property=ProtectSystem=strict \
  --property=ProtectHome=yes \
  --property=PrivateTmp=yes \
  --property=NoNewPrivileges=yes \
  --property=RestrictSUIDSGID=yes \
  --property=ProtectKernelTunables=yes \
  --property=ProtectKernelModules=yes \
  --property=ProtectKernelLogs=yes \
  --property=ProtectControlGroups=yes \
  --property=LockPersonality=yes \
  --property=MemoryDenyWriteExecute=yes \
  --property=SystemCallFilter=@system-service \
  -- \
  "$store_path/bin/listenfds"

sudo systemctl show "$activation_socket" \
  --property=ActiveState \
  --property=Listen \
  --property=Triggers
assert_property "$activation_socket" ActiveState active
if sudo systemctl is-active --quiet "$activation_service"; then
  echo "$activation_service started before the first connection" >&2
  exit 1
fi
echo "service before first connection: inactive"

page=$(curl --fail --silent --show-error "http://$listen_address/")
[[ $page == "LISTEN_FDS=1; no socket() authority" ]]
echo "first connection activated service: $page"

for _ in {1..50}; do
  sudo systemctl is-active --quiet "$activation_service" && break
  sleep 0.1
done
sudo systemctl show "$activation_service" \
  --property=ActiveState \
  --property=DynamicUser \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=CapabilityBoundingSet \
  --property=TriggeredBy \
  --property=Sockets
assert_property "$activation_service" ActiveState active
assert_property "$activation_service" DynamicUser yes
assert_property "$activation_service" PrivateNetwork yes
assert_property "$activation_service" RestrictAddressFamilies AF_UNIX
assert_property "$activation_service" CapabilityBoundingSet ""

second=$(curl --fail --silent --show-error "http://$listen_address/")
[[ $second == "$page" ]]
echo "second connection reused the activated service"

stop_unit "$activation_socket"
stop_unit "$activation_service"
! ss -ltn '( sport = :18081 )' | grep -q 127.0.0.1:18081
echo "stopped cleanly; activation units and listener removed"


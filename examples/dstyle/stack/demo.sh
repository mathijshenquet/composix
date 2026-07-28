#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
source "$example_dir/../lib/demo-lib.sh"

cix_bin=$(resolve_cix)
backend_unit=
nginx_unit=cix-dstyle-stack-nginx.service
edge_group=cix-dstyle-stack-edge
group_created=false
backend_dir=/run/cix-run-backend
backend_socket=$backend_dir/backend.sock
nginx_dir=/run/cix-run-stack-nginx
nginx_socket=$nginx_dir/http.sock

cleanup() {
  stop_unit "$nginx_unit"
  stop_unit "$backend_unit"
  [[ -z $backend_unit ]] || wait_for_unit_gone "$backend_unit" || true
  collect_cix_run_slice
  if $group_created; then
    sudo groupdel "$edge_group" >/dev/null 2>&1 || true
    group_created=false
  fi
  for path in "$backend_dir" "$nginx_dir"; do
    [[ ! -e $path ]] || {
      echo "cleanup failed: $path remains" >&2
      return 1
    }
  done
}
trap cleanup EXIT

sudo -n true
stop_unit "$nginx_unit"
if getent group "$edge_group" >/dev/null; then
  echo "temporary edge group $edge_group already exists" >&2
  exit 1
fi
sudo groupadd --system "$edge_group"
group_created=true

store_path=$(nix-build "$example_dir" --no-out-link)
backend_unit=$(sudo "$cix_bin" run "$store_path#backend" --detach)
echo "started $backend_unit"
wait_for_path "$backend_socket"

sudo systemctl show "$backend_unit" \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=RuntimeDirectory \
  --property=RuntimeDirectoryMode \
  --property=DynamicUser
assert_property "$backend_unit" PrivateNetwork yes
assert_property "$backend_unit" RestrictAddressFamilies AF_UNIX
assert_property "$backend_unit" RuntimeDirectory cix-run-backend
assert_property "$backend_unit" RuntimeDirectoryMode 0700
assert_property "$backend_unit" DynamicUser yes

if sudo setpriv --reuid=1001 --regid=1001 --clear-groups \
  test -S "$backend_socket" 2>/dev/null; then
  echo "uid 1001 unexpectedly traversed the backend runtime directory" >&2
  exit 1
fi
echo "ungranted consumer: backend socket is hidden by the producer's 0700 directory"

sudo chgrp "$edge_group" "$backend_dir" "$backend_socket"
sudo chmod 2750 "$backend_dir"
sudo chmod 0660 "$backend_socket"

sudo systemd-run \
  --unit="$nginx_unit" \
  --collect \
  --service-type=exec \
  --property=DynamicUser=yes \
  --property=SupplementaryGroups="$edge_group" \
  --property=RuntimeDirectory=cix-run-stack-nginx:nginx \
  --property=RuntimeDirectoryMode=0700 \
  --property=CacheDirectory=cix-run-stack-nginx:nginx \
  --property=CacheDirectoryMode=0700 \
  --property=BindPaths="$backend_dir:/run/stack-shared" \
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
  --property=CapabilityBoundingSet= \
  --property=RestrictAddressFamilies=AF_UNIX \
  --property=PrivateNetwork=yes \
  -- \
  "$store_path/bin/nginx" -c "$store_path/nginx.conf" -e stderr

wait_for_path "$nginx_socket"
sudo systemctl show "$nginx_unit" \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=RuntimeDirectory \
  --property=RuntimeDirectoryMode \
  --property=DynamicUser \
  --property=SupplementaryGroups \
  --property=BindPaths
assert_property "$nginx_unit" PrivateNetwork yes
assert_property "$nginx_unit" RestrictAddressFamilies AF_UNIX
assert_property "$nginx_unit" RuntimeDirectory cix-run-stack-nginx
assert_property "$nginx_unit" RuntimeDirectoryMode 0700
assert_property "$nginx_unit" DynamicUser yes
assert_property "$nginx_unit" SupplementaryGroups "$edge_group"
assert_property "$nginx_unit" BindPaths "$backend_dir:/run/stack-shared:rbind"

page=$(sudo curl --fail --silent --show-error --unix-socket "$nginx_socket" http://localhost/)
[[ $page == "hello from the dstyle backend" ]]
echo "browser -> nginx unix socket -> backend unix socket: $page"

stop_unit "$nginx_unit"
stop_unit "$backend_unit"
wait_for_unit_gone "$backend_unit"
backend_unit=
collect_cix_run_slice
sudo groupdel "$edge_group"
group_created=false
[[ ! -e $backend_dir ]]
[[ ! -e $nginx_dir ]]
echo "stopped cleanly; services, sockets, runtime paths, and edge group removed"

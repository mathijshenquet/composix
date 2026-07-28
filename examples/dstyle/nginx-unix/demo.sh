#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
source "$example_dir/../lib/demo-lib.sh"

cix_bin=$(resolve_cix)
nginx_unit=
publish_base=cix-dstyle-nginx-publish
publish_service=$publish_base.service
publish_socket=$publish_base.socket
host_socket_dir=/run/cix-run-nginx
host_socket=$host_socket_dir/http.sock
proxyd=/usr/lib/systemd/systemd-socket-proxyd

cleanup() {
  stop_unit "$publish_socket"
  stop_unit "$publish_service"
  stop_unit "$nginx_unit"
  [[ -z $nginx_unit ]] || wait_for_unit_gone "$nginx_unit" || true
  collect_cix_run_slice
  [[ ! -e $host_socket_dir ]] || {
    echo "cleanup failed: $host_socket_dir remains" >&2
    return 1
  }
  if ss -ltn '( sport = :8080 )' | grep -q 127.0.0.1:8080; then
    echo "cleanup failed: publisher still listens on 127.0.0.1:8080" >&2
    return 1
  fi
}
trap cleanup EXIT

sudo -n true
[[ -x $proxyd ]]
stop_unit "$publish_socket"
stop_unit "$publish_service"
if ss -ltn '( sport = :8080 )' | grep -q 127.0.0.1:8080; then
  echo "127.0.0.1:8080 is already in use" >&2
  exit 1
fi

store_path=$(nix-build "$example_dir" --no-out-link)
nginx_unit=$(sudo "$cix_bin" run "$store_path" --detach)
echo "started $nginx_unit"
wait_for_path "$host_socket"

sudo systemctl show "$nginx_unit" \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=RuntimeDirectory \
  --property=RuntimeDirectoryMode \
  --property=DynamicUser
assert_property "$nginx_unit" PrivateNetwork yes
assert_property "$nginx_unit" RestrictAddressFamilies AF_UNIX
assert_property "$nginx_unit" RuntimeDirectory cix-run-nginx
assert_property "$nginx_unit" RuntimeDirectoryMode 0700
assert_property "$nginx_unit" DynamicUser yes

page=$(sudo curl --fail --silent --show-error --unix-socket "$host_socket" http://localhost/)
[[ $page == *"hello from dstyle nginx"* ]]
echo "direct curl --unix-socket: $page"

sudo systemd-run \
  --unit="$publish_base" \
  --collect \
  --service-type=notify \
  --socket-property=ListenStream=127.0.0.1:8080 \
  --property=PrivateNetwork=yes \
  --property=RestrictAddressFamilies=AF_UNIX \
  --property=BindReadOnlyPaths="$host_socket_dir" \
  --property=ProtectSystem=strict \
  --property=NoNewPrivileges=yes \
  -- \
  "$proxyd" "$host_socket"

sudo systemctl show "$publish_socket" \
  --property=ActiveState \
  --property=Listen \
  --property=Triggers
assert_property "$publish_socket" ActiveState active
if sudo systemctl is-active --quiet "$publish_service"; then
  echo "$publish_service started before the first connection" >&2
  exit 1
fi
echo "publisher service before first connection: inactive"

page=$(curl --fail --silent --show-error http://127.0.0.1:8080/)
[[ $page == *"hello from dstyle nginx"* ]]
echo "published TCP request: $page"

for _ in {1..50}; do
  sudo systemctl is-active --quiet "$publish_service" && break
  sleep 0.1
done
sudo systemctl show "$publish_service" \
  --property=ActiveState \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=TriggeredBy
assert_property "$publish_service" ActiveState active
assert_property "$publish_service" PrivateNetwork yes
assert_property "$publish_service" RestrictAddressFamilies AF_UNIX

stop_unit "$publish_socket"
stop_unit "$publish_service"
stop_unit "$nginx_unit"
wait_for_unit_gone "$nginx_unit"
nginx_unit=
collect_cix_run_slice
[[ ! -e $host_socket_dir ]]
! ss -ltn '( sport = :8080 )' | grep -q 127.0.0.1:8080
echo "stopped cleanly; transient publisher and unix socket removed"


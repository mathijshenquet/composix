#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
source "$example_dir/../lib/demo-lib.sh"

cix_bin=$(resolve_cix)
unit=
peer_unit=cix-dstyle-postgres-peer.service
host_socket_dir=/run/cix-run-postgres

cleanup() {
  stop_unit "$peer_unit"
  stop_unit "$unit"
  [[ -z $unit ]] || wait_for_unit_gone "$unit" || true
  collect_cix_run_slice
  [[ ! -e $host_socket_dir ]] || {
    echo "cleanup failed: $host_socket_dir remains" >&2
    return 1
  }
}
trap cleanup EXIT

sudo -n true
stop_unit "$peer_unit"

store_path=$(nix-build "$example_dir" --no-out-link)
unit=$(sudo "$cix_bin" run "$store_path" --detach)
echo "started $unit"

wait_for_path "$host_socket_dir/.s.PGSQL.5432"

sudo systemctl show "$unit" \
  --property=PrivateNetwork \
  --property=RestrictAddressFamilies \
  --property=RuntimeDirectory \
  --property=RuntimeDirectoryMode \
  --property=DynamicUser \
  --property=User
assert_property "$unit" PrivateNetwork yes
assert_property "$unit" RestrictAddressFamilies AF_UNIX
assert_property "$unit" RuntimeDirectory cix-run-postgres
assert_property "$unit" RuntimeDirectoryMode 0700
assert_property "$unit" DynamicUser yes

echo "host socket directory:"
sudo stat --format='mode=%a uid=%u gid=%g path=%n' "$host_socket_dir"

query=$(
  sudo "$store_path/bin/psql" \
    --host="$host_socket_dir" \
    --username=cix \
    --dbname=postgres \
    --no-password \
    --tuples-only \
    --no-align \
    --command='SELECT 1'
)
[[ $query == "1" ]]
echo "root host client: SELECT $query"

if sudo setpriv --reuid=1001 --regid=1001 --clear-groups \
  "$store_path/bin/psql" \
  --host="$host_socket_dir" \
  --username=cix \
  --dbname=postgres \
  --no-password \
  --command='SELECT 1' >/dev/null 2>&1; then
  echo "uid 1001 unexpectedly reached the PostgreSQL socket" >&2
  exit 1
fi
echo "uid 1001 host client: denied by the 0700 runtime directory"

if sudo systemd-run \
  --unit="$peer_unit" \
  --collect \
  --wait \
  --service-type=exec \
  --property=DynamicUser=yes \
  --property=PrivateNetwork=yes \
  --property=RestrictAddressFamilies=AF_UNIX \
  --property=BindReadOnlyPaths="$host_socket_dir" \
  -- \
  "$store_path/bin/psql" \
  --host="$host_socket_dir" \
  --username=cix \
  --dbname=postgres \
  --no-password \
  --command='SELECT 1' >/dev/null 2>&1; then
  echo "unrelated DynamicUser unexpectedly reached the PostgreSQL socket" >&2
  exit 1
fi
echo "unrelated DynamicUser client: denied despite receiving the socket path"

sudo systemctl stop "$unit"
wait_for_unit_gone "$unit"
unit=
collect_cix_run_slice
[[ ! -e $host_socket_dir ]]
echo "stopped cleanly; runtime directory removed"

#!/usr/bin/env bash
set -euo pipefail

stack_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$stack_dir/../../.." && pwd)
cix_bin=${CIX_BIN:-"$repo_root/target/debug/cix"}
cix_bin=$(realpath "$cix_bin")
work_dir=$(mktemp -d)
state_dir=$(mktemp -d)
address=127.0.0.1:8080

root_cix() {
  sudo env \
    CIX_STATE_DIR="$state_dir" \
    PATH="/nix/var/nix/profiles/default/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    "$cix_bin" "$@"
}

cleanup() {
  root_cix down stack >/dev/null 2>&1 || true
  sudo systemctl stop 'cix-stack*' >/dev/null 2>&1 || true
  sudo systemctl reset-failed 'cix-stack*' >/dev/null 2>&1 || true
  sudo systemctl daemon-reload >/dev/null 2>&1 || true
  sudo rm -rf -- "$work_dir" "$state_dir"
}
trap cleanup EXIT

wait_for_page() {
  local expected=$1 page=
  for _ in {1..300}; do
    if page=$(curl --fail --silent --show-error "http://$address/" 2>/dev/null) &&
      [[ $page == "$expected" ]]; then
      printf '%s\n' "$page"
      return 0
    fi
    sleep 0.1
  done
  sudo journalctl --no-pager -n 80 -u 'cix-stack-*' >&2 || true
  echo "timed out waiting for $expected" >&2
  return 1
}

assert_manager_clean() {
  local manager=$1
  local -a command=(systemctl)
  [[ $manager == system ]] || command+=(--user)
  if "${command[@]}" list-units 'cix-stack*' --all --no-legend --plain | grep -q .; then
    "${command[@]}" list-units 'cix-stack*' --all --no-legend --plain >&2
    echo "$manager manager retained stack units" >&2
    return 1
  fi
}

sudo -n true
[[ -x $cix_bin ]]
root_cix down stack >/dev/null 2>&1 || true
assert_manager_clean system
assert_manager_clean user

cp -a "$stack_dir/backend" "$stack_dir/web" "$work_dir/"
cp "$stack_dir/compose.json" "$stack_dir/generate.py" "$work_dir/"
python3 "$work_dir/generate.py" >"$work_dir/generated.json"
cmp "$work_dir/compose.json" "$work_dir/generated.json"

db_path=$(nix-build "$repo_root/examples/pack/redis" --no-out-link)
backend_v1=$("$cix_bin" build "$work_dir/backend#backend")
web_path=$("$cix_bin" build "$work_dir/web#nginx")
cp -a "$work_dir/backend" "$work_dir/backend-v2"
printf 'hello from backend v2\n' >"$work_dir/backend-v2/greeting.txt"
backend_v2=$("$cix_bin" build "$work_dir/backend-v2#backend")

root_cix tag "$db_path" stack-db:v1
root_cix tag "$backend_v1" stack-backend:current
root_cix tag "$web_path" stack-web:v1
root_cix compose check "$work_dir/compose.json"
root_cix up "$work_dir/compose.json"

v1='hello from backend v1 via compose: PONG'
wait_for_page "$v1"
root_cix ps | grep -F 'stack' | grep -F 'backend'
[[ $(sudo systemctl show cix-stack-web.service -p PrivateNetwork --value) == yes ]]
[[ $(sudo systemctl show cix-stack-web.service -p RestrictAddressFamilies --value) == AF_UNIX ]]
[[ $(sudo systemctl show cix-stack-web.service -p SocketBindDeny --value) == any ]]
sudo systemctl cat cix-stack-web-http.socket | grep -Fx 'FileDescriptorName=http'

web_before=$(sudo systemctl show cix-stack-web.service -p ActiveEnterTimestampMonotonic --value)
db_before=$(sudo systemctl show cix-stack-db.service -p ActiveEnterTimestampMonotonic --value)
backend_before=$(sudo systemctl show cix-stack-backend.service -p ActiveEnterTimestampMonotonic --value)

root_cix tag "$backend_v2" stack-backend:current
diff_output=$(root_cix compose diff "$work_dir/compose.json")
printf '%s\n' "$diff_output"
grep -Fx 'unit changed: cix-stack-backend.service' <<<"$diff_output"
grep -F "service backend: $backend_v1 -> $backend_v2" <<<"$diff_output"
[[ $(grep -c '^service ' <<<"$diff_output") == 1 ]]
! grep -Eq 'unit changed: cix-stack-(web|db)\.service' <<<"$diff_output"

root_cix up "$work_dir/compose.json"
[[ $(sudo systemctl show cix-stack-web.service -p ActiveEnterTimestampMonotonic --value) == "$web_before" ]]
[[ $(sudo systemctl show cix-stack-db.service -p ActiveEnterTimestampMonotonic --value) == "$db_before" ]]
[[ $(sudo systemctl show cix-stack-backend.service -p ActiveEnterTimestampMonotonic --value) != "$backend_before" ]]
v2='hello from backend v2 via compose: PONG'
wait_for_page "$v2"

root_cix rollback stack
wait_for_page "$v1"
root_cix down stack
sudo systemctl reset-failed 'cix-stack*' >/dev/null 2>&1 || true

! ss -ltn '( sport = :8080 )' | grep -q "$address"
for path in /run/redis /run/backend; do
  [[ ! -e $path ]]
done
for link in /etc/systemd/system/cix-stack*; do
  [[ ! -e $link && ! -L $link ]]
done
assert_manager_clean system
assert_manager_clean user
echo "compose demo passed: fd-only web -> backend -> db, selective update, rollback, clean down"

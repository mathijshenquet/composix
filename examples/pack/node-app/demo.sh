#!/usr/bin/env bash
set -euo pipefail

example_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cix_bin=$(realpath "${CIX_BIN:-$(command -v cix)}")
no_jit_dir=$(mktemp -d)
no_jit_unit=
unit=

cleanup() {
  if [[ -n "$no_jit_unit" ]]; then
    sudo systemctl stop "$no_jit_unit" >/dev/null 2>&1 || true
  fi
  if [[ -n "$unit" ]]; then
    sudo systemctl stop "$unit" >/dev/null 2>&1 || true
  fi
  rm -rf "$no_jit_dir"
}
trap cleanup EXIT

cp "$example_dir/Cixfile" "$no_jit_dir/Cixfile"
cp "$example_dir/Cixfile.lock" "$no_jit_dir/Cixfile.lock"
cp "$example_dir/server.js" "$no_jit_dir/server.js"
sed -i '/^GRANT jit$/d' "$no_jit_dir/Cixfile"

no_jit_store=$("$cix_bin" build "$no_jit_dir")
no_jit_unit=$(sudo "$cix_bin" run "$no_jit_store" --detach)
echo "started no-JIT control $no_jit_unit"
[[ $(sudo systemctl show "$no_jit_unit" --property=MemoryDenyWriteExecute --value) == yes ]]

for _ in {1..100}; do
  if sudo journalctl --unit "$no_jit_unit" --no-pager | grep -Fq 'V8_Fatal'; then
    v8_failed=true
    break
  fi
  sleep 0.1
done
[[ ${v8_failed:-} == true ]]
sudo journalctl --unit "$no_jit_unit" --no-pager | tail -n 20
sudo systemctl stop "$no_jit_unit"
no_jit_unit=

store_path=$("$cix_bin" build "$example_dir")
unit=$(sudo "$cix_bin" run "$store_path" --detach)
echo "started $unit"
[[ $(sudo systemctl show "$unit" --property=MemoryDenyWriteExecute --value) != yes ]]

for _ in {1..100}; do
  if page=$(curl --fail --silent http://127.0.0.1:8081/ 2>/dev/null); then
    break
  fi
  sleep 0.1
done
grep -F 'node JIT is enabled' <<<"$page"

"$cix_bin" ps
sudo systemctl stop "$unit"
echo "stopped $unit"
unit=

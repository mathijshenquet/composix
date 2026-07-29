#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d)"
unit=""
host_state=""

cleanup() {
  if [[ -n "$unit" ]]; then
    systemctl --user stop "$unit" >/dev/null 2>&1 || true
  fi
  if [[ -n "$host_state" ]]; then
    rm -f -- "$host_state/timestamp"
    rmdir -- "$host_state" >/dev/null 2>&1 || true
  fi
  rm -rf -- "$fixture"
}
trap cleanup EXIT

host_state="${XDG_STATE_HOME:-$HOME/.local/state}/cix-run-demo"
app_state="$host_state"
rm -f -- "$host_state/timestamp"
mkdir -p "$fixture/output/bin"
shell="$(command -v sh)"
date_bin="$(command -v date)"
sleep_bin="$(command -v sleep)"

cat >"$fixture/output/bin/service" <<EOF
#!$shell
set -eu
$date_bin --iso-8601=seconds > "$app_state/timestamp"
exec $sleep_bin 300
EOF
chmod +x "$fixture/output/bin/service"

cat >"$fixture/output/cix-manifest.json" <<EOF
{
  "cixManifest": 1,
  "services": {
    "demo": {
      "exec": ["bin/service"],
      "dirs": {"state": ["$app_state"]}
    }
  }
}
EOF

store_path="$(nix store add-path "$fixture/output")"
echo "fixture: $store_path"
unit="$(
  cd "$repo_root"
  cargo run --quiet -p cix -- run "$store_path#demo" --user --detach
)"
echo "unit: $unit"

for _ in {1..100}; do
  [[ -f "$host_state/timestamp" ]] && break
  sleep 0.1
done
[[ -f "$host_state/timestamp" ]]

echo "timestamp in managed state: $(<"$host_state/timestamp")"
(
  cd "$repo_root"
  cargo run --quiet -p cix -- ps
)
systemctl --user stop "$unit"
unit=""
echo "stopped cleanly"

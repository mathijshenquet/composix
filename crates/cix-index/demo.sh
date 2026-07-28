#!/usr/bin/env bash
set -euo pipefail

root_dir=$(mktemp -d)
server_state="$root_dir/server"
client_state="$root_dir/client"
source_dir="$root_dir/source"
port="${PORT:-18420}"
root_url="localhost:$port"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then kill "$server_pid" 2>/dev/null || true; fi
  rm -rf "$root_dir"
}
trap cleanup EXIT

mkdir -p "$source_dir/input"
printf 'hello from composix index\n' > "$source_dir/input/hello"

echo '1. Add a small directory to the Nix store'
store_path=$(nix store add-path "$source_dir/input")
printf '   %s\n' "$store_path"

echo '2. Tag it in a publisher state directory'
CIX_STATE_DIR="$server_state" cargo run -q -p cix -- tag "$store_path" "$root_url/x:v1"

echo '3. Serve the resolver and file:// binary cache'
CIX_STATE_DIR="$server_state" cargo run -q -p cix -- serve "$root_url" --listen "127.0.0.1:$port" --with-store > "$root_dir/server.log" 2>&1 &
server_pid=$!
sleep 1

echo '4. Resolve and pull into a separate client state directory'
curl --fail --silent "http://127.0.0.1:$port/v1/resolve/x/v1"
printf '\n'
CIX_STATE_DIR="$client_state" cargo run -q -p cix -- pull "$root_url/x:v1" --as x

echo '5. Inspect the pulled local tag (including its upstream)'
CIX_STATE_DIR="$client_state" cargo run -q -p cix -- ls -l

#!/usr/bin/env bash

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
prepared="$here/.prepared"
mkdir -p "$prepared/cargo-home"

nix_out() {
  nix build --no-link --json "nixpkgs#$1" | jq -r '.[0].outputs.out'
}

bash_path=$(nix_out bashInteractive)
coreutils_path=$(nix_out coreutils)
strace_path=$(nix_out strace)
cargo_chef_path=$(nix_out cargo-chef)
cargo_path=$(nix_out cargo)
rustc_path=$(nix_out rustc)
cc_path=$(nix_out gcc)
tool_path="$bash_path/bin:$coreutils_path/bin:$strace_path/bin:$cargo_chef_path/bin:$cargo_path/bin:$rustc_path/bin:$cc_path/bin"

(
  cd "$here"
  env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
    PATH="$tool_path" CARGO_HOME="$prepared/cargo-home" cargo generate-lockfile
  env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
    PATH="$tool_path" CARGO_HOME="$prepared/cargo-home" cargo fetch --locked
  env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
    PATH="$tool_path" CARGO_HOME="$prepared/cargo-home" \
    cargo chef prepare --recipe-path recipe.json
)

jq -n \
  --arg bash "$bash_path" \
  --arg coreutils "$coreutils_path" \
  --arg strace "$strace_path" \
  --arg cargoChef "$cargo_chef_path" \
  --arg cargo "$cargo_path" \
  --arg rustc "$rustc_path" \
  --arg cc "$cc_path" \
  '{bash: $bash, coreutils: $coreutils, strace: $strace, cargo_chef: $cargoChef,
    cargo: $cargo, rustc: $rustc, cc: $cc}' >"$prepared/tools.json"

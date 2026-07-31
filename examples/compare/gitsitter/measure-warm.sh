#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
example_root="$repo_root/examples/compare/gitsitter"
cix_bin=${CIX_BIN:-"$repo_root/target/debug/cix"}
upstream_ref=github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd
upstream_src=$(nix flake archive --json "$upstream_ref" | jq -r .path)
bench_root=$(mktemp -d /tmp/composix-nixcompare.XXXXXX)
trap 'rm -rf -- "$bench_root"' EXIT

cp -R "$upstream_src" "$bench_root/gitsitter"
chmod -R u+w "$bench_root/gitsitter"
patch -s -d "$bench_root/gitsitter" -p1 < "$example_root/warm.patch"

nix build "$upstream_ref" --no-link >/dev/null 2>&1
nix build "path:$example_root/crane" --no-link >/dev/null 2>&1
nix build --impure --no-link --expr \
  "let upstream = builtins.getFlake \"$upstream_ref\"; in upstream.packages.x86_64-linux.default.overrideAttrs (_: { src = $bench_root/gitsitter; GIT_COMMIT_HASH = \"29c8a2d\"; })" \
  >/dev/null 2>&1
nix build "path:$example_root/crane" \
  --override-input gitsitter "path:$bench_root/gitsitter" --no-link >/dev/null 2>&1

/usr/bin/time -o "$bench_root/upstream.time" -f 'upstream_warm_change_seconds=%e' \
  nix build --rebuild --impure --no-link --expr \
  "let upstream = builtins.getFlake \"$upstream_ref\"; in upstream.packages.x86_64-linux.default.overrideAttrs (_: { src = $bench_root/gitsitter; GIT_COMMIT_HASH = \"29c8a2d\"; })" \
  >/dev/null 2>&1
cat "$bench_root/upstream.time"

/usr/bin/time -o "$bench_root/crane.time" -f 'crane_warm_change_seconds=%e' \
  nix build --rebuild "path:$example_root/crane" \
  --override-input gitsitter "path:$bench_root/gitsitter" --no-link \
  >/dev/null 2>&1
cat "$bench_root/crane.time"

cp -R "$example_root/cix" "$bench_root/cix"
cp "$upstream_src/Cargo.toml" "$upstream_src/Cargo.lock" \
  "$upstream_src/build.rs" "$bench_root/cix/"
cp -R "$upstream_src/src" "$bench_root/cix/src"
chmod -R u+w "$bench_root/cix"
sed -i 's|^FROM github:mathijshenquet/gitsitter AS src$|FROM . AS src|' \
  "$bench_root/cix/Cixfile"
jq 'del(.memo)' "$bench_root/cix/Cixfile.lock" > "$bench_root/Cixfile.lock"
mv "$bench_root/Cixfile.lock" "$bench_root/cix/Cixfile.lock"
CIX_BUILD_WORKSPACE_DIR="$bench_root/workspaces" \
  "$cix_bin" build "$bench_root/cix#gitsitter" >/dev/null 2>&1
patch -s -d "$bench_root/cix" -p1 < "$example_root/warm.patch"
/usr/bin/time -o "$bench_root/cix.time" -f 'cix_warm_change_seconds=%e' \
  env CIX_BUILD_WORKSPACE_DIR="$bench_root/workspaces" \
  "$cix_bin" build "$bench_root/cix#gitsitter" >/dev/null 2>&1
cat "$bench_root/cix.time"

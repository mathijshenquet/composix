#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
example_root="$repo_root/examples/compare/gitsitter"
cix_bin=${CIX_BIN:-"$repo_root/target/debug/cix"}
upstream_ref=github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd
upstream_src=$(nix flake archive --json "$upstream_ref" | jq -r .path)
bench_root=$(mktemp -d "${TMPDIR:-/tmp}/composix-nixcompare.XXXXXX")
if [[ ${CIX_KEEP_BENCH:-0} == 1 ]]; then
  trap 'printf "cix_benchmark_root=%s\\n" "$bench_root"' EXIT
else
  trap 'rm -rf -- "$bench_root"' EXIT
fi

cp -R "$upstream_src" "$bench_root/gitsitter"
chmod -R u+w "$bench_root/gitsitter"
patch -s -d "$bench_root/gitsitter" -p1 < "$example_root/warm.patch"

if [[ ${CIX_ONLY:-0} != 1 ]]; then
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
fi

cp -R "$example_root/cix" "$bench_root/cix"
cp -R "$upstream_src"/. "$bench_root/cix/"
chmod -R u+w "$bench_root/cix"
sed -i 's|^FROM github:mathijshenquet/gitsitter AS src$|FROM . AS src|' \
  "$bench_root/cix/Cixfile"
sed -i 's|^  ENV GIT_COMMIT_HASH = ${src.rev}$|  ENV GIT_COMMIT_HASH = 29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd|' \
  "$bench_root/cix/Cixfile"
jq 'del(.memo, .outputs)' "$bench_root/cix/Cixfile.lock" > "$bench_root/Cixfile.lock"
mv "$bench_root/Cixfile.lock" "$bench_root/cix/Cixfile.lock"
CIX_BUILD_WORKSPACE_DIR="$bench_root/workspaces" \
  "$cix_bin" build "$bench_root/cix#gitsitter" >/dev/null 2>&1
cp "$bench_root/cix/Cixfile.lock" "$bench_root/warm-prime.lock"
# Run cold from only the pin: neither recorded step memo is eligible to carry
# warm self-observation into this control. FETCH therefore restores its pinned
# snapshot and RUN executes from that reconstructed workspace.
jq 'del(.memo, .outputs, .stepMemo["builder:build:2"], .stepMemo["builder:build:3"])' \
  "$bench_root/cix/Cixfile.lock" > "$bench_root/Cixfile.lock"
mv "$bench_root/Cixfile.lock" "$bench_root/cix/Cixfile.lock"
CIX_BUILD_WORKSPACE_DIR="$bench_root/workspaces" \
  "$cix_bin" build --cold "$bench_root/cix#gitsitter" >/dev/null 2>&1
mv "$bench_root/warm-prime.lock" "$bench_root/cix/Cixfile.lock"
printf 'cix_cold_control=green\n'
patch -s -d "$bench_root/cix" -p1 < "$example_root/warm.patch"
profile_bin="$bench_root/cix-profile-bin"
profile_log="$bench_root/cix-nix-subprocesses.tsv"
mkdir "$profile_bin"
for tool in nix nix-store; do
  tool_path=$(command -v "$tool")
  printf '#!/usr/bin/env bash\n/usr/bin/time -f "%%e\\t%%x\\t%%C" -o "%s" -a "%s" "$@"\n' \
    "$profile_log" "$tool_path" > "$profile_bin/$tool"
  chmod +x "$profile_bin/$tool"
done
/usr/bin/time -o "$bench_root/cix.time" -f 'cix_readset_warm_change_seconds=%e' \
  env CIX_BUILD_WORKSPACE_DIR="$bench_root/workspaces" CIX_TIMING=1 PATH="$profile_bin:$PATH" \
  "$cix_bin" build --stats "$bench_root/cix#gitsitter" \
  >"$bench_root/cix.stats" 2>"$bench_root/cix.timing"
cat "$bench_root/cix.time"
jq -c '{stats: {nixSubprocesses: .stats.nixSubprocesses, steps: [.stats.steps[] | select(.kind == "FETCH" or .kind == "RUN")]}}' \
  "$bench_root/cix.stats"
grep '^CIX timing' "$bench_root/cix.timing"
printf 'cix_run_compiled_units=%s\n' "$(grep -c '^   Compiling' "$bench_root/cix.timing" || true)"
nl -ba "$profile_log" | sed 's/^/cix_nix_subprocess\t/'

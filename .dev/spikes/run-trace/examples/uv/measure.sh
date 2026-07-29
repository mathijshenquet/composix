#!/usr/bin/env bash

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
spike=$(cd "$here/../.." && pwd)
harness="$spike/harness/runtrace"
tools="$here/.prepared/tools.json"
results="$spike/results/uv"
session=$(mktemp -d "$spike/work/uv-measure.XXXXXX")
mkdir -p "$results"

mapfile -t offers < <(jq -r '.[]' "$tools")
path=$(jq -r '[.[] + "/bin"] | join(":")' "$tools")
python=$(jq -r .python "$tools")
declare -a offer_args=()
for offer in "${offers[@]}"; do offer_args+=(--offer "$offer"); done

make_work() {
  local destination=$1
  mkdir -p "$destination/.uv-cache"
  cp -a pyproject.toml uv.lock src "$destination/"
  cp -a "$here/.prepared/cache/." "$destination/.uv-cache/"
}

run_sync() {
  local work=$1 output=$2 memo=$3 trace=$4
  shift 4
  "$harness" "$@" --workdir "$work" --output "$output" --memo "$memo" \
    --trace-prefix "$trace" "${offer_args[@]}" \
    --env "PATH=$path" --env UV_CACHE_DIR=/work/.uv-cache \
    --env "UV_PYTHON=$python/bin/python" --env UV_PYTHON_DOWNLOADS=never \
    --env UV_PROJECT_ENVIRONMENT=/out/venv --env UV_LINK_MODE=copy \
    -- 'uv sync --offline && /out/venv/bin/python -c "import runtrace_uv; print(runtrace_uv.proof())" > /out/proof.txt'
}

cd "$here"
for name in first hit second miss untraced; do make_work "$session/$name/work"; done
run_sync "$session/first/work" "$session/first/out" "$session/memo.json" \
  "$session/first/trace" >"$session/first.log"
cp "$session/memo.json" "$session/first-memo.json"
run_sync "$session/hit/work" "$session/hit/out" "$session/memo.json" \
  "$session/hit/trace" >"$session/hit.json"
run_sync "$session/second/work" "$session/second/out" "$session/memo.json" \
  "$session/second/trace" --force >"$session/second.log"
cp "$session/memo.json" "$session/second-memo.json"
sed -i 's#example.invalid/d38#example.invalid/d39#' \
  "$session/miss/work/src/runtrace_uv/__init__.py"
run_sync "$session/miss/work" "$session/miss/out" "$session/memo.json" \
  "$session/miss/trace" >"$session/miss.log"
cp "$session/memo.json" "$session/miss-memo.json"
run_sync "$session/untraced/work" "$session/untraced/out" "$session/no-memo.json" \
  "$session/untraced/trace" --force --no-trace >"$session/untraced.log"

cp "$session/first/trace.store-paths" "$results/trace-1.store-paths"
cp "$session/second/trace.store-paths" "$results/trace-2.store-paths"
cp "$session/miss/trace.store-paths" "$results/trace-miss.store-paths"
cp "$session/first/trace.non-store-summary" "$results/trace-1.non-store-summary"
jq -n \
  --argjson first "$(jq . "$session/first-memo.json")" \
  --argjson second "$(jq . "$session/second-memo.json")" \
  --argjson miss "$(jq . "$session/miss-memo.json")" \
  --argjson hit "$(jq . "$session/hit.json")" \
  --argjson untraced "$(tail -n 1 "$session/untraced.log")" \
  --arg stable "$(cmp -s "$results/trace-1.store-paths" "$results/trace-2.store-paths" && echo true || echo false)" \
  '{first: $first, second: $second, memo_hit: $hit, miss: $miss,
    closure_stable: ($stable == "true"),
    output_stable: ($first.output_hash == $second.output_hash),
    miss_output_changed: ($second.output_hash != $miss.output_hash),
    miss_trace_changed: ($second.traced_store_paths != $miss.traced_store_paths),
    untraced: $untraced}' >"$results/summary.json"

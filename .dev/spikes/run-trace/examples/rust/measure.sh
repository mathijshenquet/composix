#!/usr/bin/env bash

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
spike=$(cd "$here/../.." && pwd)
harness="$spike/harness/runtrace"
tools="$here/.prepared/tools.json"
results="$spike/results/rust"
session=$(mktemp -d "$spike/work/rust-measure.XXXXXX")
mkdir -p "$results"

mapfile -t offers < <(jq -r '.[]' "$tools")
path=$(jq -r '[.[] + "/bin"] | join(":")' "$tools")
declare -a offer_args=()
for offer in "${offers[@]}"; do
  offer_args+=(--offer "$offer")
done

make_work() {
  local destination=$1
  mkdir -p "$destination"
  cp -a Cargo.toml Cargo.lock recipe.json src "$destination/"
}

run_cook() {
  local work=$1 output=$2 memo=$3 trace=$4
  shift 4
  "$harness" "$@" --workdir "$work" --output "$output" --memo "$memo" \
    --trace-prefix "$trace" "${offer_args[@]}" \
    --dep "$here/.prepared/cargo-home:/deps/cargo-home" \
    --env "PATH=$path" --env CARGO_HOME=/deps/cargo-home \
    --env CARGO_TARGET_DIR=/out/target \
    -- 'cargo chef cook --recipe-path recipe.json'
}

run_build() {
  local work=$1 output=$2 memo=$3 trace=$4 seed=$5
  shift 5
  "$harness" "$@" --workdir "$work" --output "$output" --memo "$memo" \
    --trace-prefix "$trace" "${offer_args[@]}" --seed "$seed" \
    --dep "$here/.prepared/cargo-home:/deps/cargo-home" \
    --env "PATH=$path" --env CARGO_HOME=/deps/cargo-home \
    --env CARGO_TARGET_DIR=/out/target \
    -- 'cargo build --release'
}

cd "$here"
for name in cook1 cook_hit cook2; do make_work "$session/$name/work"; done
run_cook "$session/cook1/work" "$session/cook1/out" "$session/cook.memo" \
  "$session/cook1/trace" >"$session/cook1.log"
cp "$session/cook.memo" "$session/cook1-memo.json"
run_cook "$session/cook_hit/work" "$session/cook_hit/out" "$session/cook.memo" \
  "$session/cook-hit/trace" >"$session/cook-hit.json"
run_cook "$session/cook2/work" "$session/cook2/out" "$session/cook.memo" \
  "$session/cook2/trace" --force >"$session/cook2.log"
cp "$session/cook.memo" "$session/cook2-memo.json"
seed=$(jq -r .output_store_path "$session/cook.memo")

for name in build1 build_hit build2 build_miss build_untraced; do
  make_work "$session/$name/work"
done
run_build "$session/build1/work" "$session/build1/out" "$session/build.memo" \
  "$session/build1/trace" "$seed" >"$session/build1.log"
cp "$session/build.memo" "$session/build1-memo.json"
run_build "$session/build_hit/work" "$session/build_hit/out" "$session/build.memo" \
  "$session/build-hit/trace" "$seed" >"$session/build-hit.json"
run_build "$session/build2/work" "$session/build2/out" "$session/build.memo" \
  "$session/build2/trace" "$seed" --force >"$session/build2.log"
cp "$session/build.memo" "$session/build2-memo.json"
sed -i 's/value: 38/value: 39/' "$session/build_miss/work/src/main.rs"
run_build "$session/build_miss/work" "$session/build_miss/out" "$session/build.memo" \
  "$session/build-miss/trace" "$seed" >"$session/build-miss.log"
cp "$session/build.memo" "$session/build-miss-memo.json"
run_build "$session/build_untraced/work" "$session/build_untraced/out" \
  "$session/untraced.memo" "$session/build-untraced/trace" "$seed" \
  --force --no-trace >"$session/build-untraced.log"

cp "$session/cook1/trace.store-paths" "$results/cook-1.store-paths"
cp "$session/cook2/trace.store-paths" "$results/cook-2.store-paths"
cp "$session/cook1/trace.non-store-summary" "$results/cook-1.non-store-summary"
cp "$session/build1/trace.store-paths" "$results/build-1.store-paths"
cp "$session/build2/trace.store-paths" "$results/build-2.store-paths"
cp "$session/build-miss/trace.store-paths" "$results/build-miss.store-paths"
cp "$session/build1/trace.non-store-summary" "$results/build-1.non-store-summary"

jq -n \
  --argjson cook1 "$(jq . "$session/cook1-memo.json")" \
  --argjson cook2 "$(jq . "$session/cook2-memo.json")" \
  --argjson cookHit "$(jq . "$session/cook-hit.json")" \
  --argjson build1 "$(jq . "$session/build1-memo.json")" \
  --argjson build2 "$(jq . "$session/build2-memo.json")" \
  --argjson buildMiss "$(jq . "$session/build-miss-memo.json")" \
  --argjson buildHit "$(jq . "$session/build-hit.json")" \
  --argjson untraced "$(tail -n 1 "$session/build-untraced.log")" \
  --arg cookClosureStable "$(cmp -s "$results/cook-1.store-paths" "$results/cook-2.store-paths" && echo true || echo false)" \
  --arg buildClosureStable "$(cmp -s "$results/build-1.store-paths" "$results/build-2.store-paths" && echo true || echo false)" \
  '{
    cook: {
      first: $cook1,
      second: $cook2,
      memo_hit: $cookHit,
      closure_stable: ($cookClosureStable == "true"),
      output_stable: ($cook1.output_hash == $cook2.output_hash)
    },
    build: {
      first: $build1,
      second: $build2,
      memo_hit: $buildHit,
      miss: $buildMiss,
      closure_stable: ($buildClosureStable == "true"),
      output_stable: ($build1.output_hash == $build2.output_hash),
      miss_output_changed: ($build2.output_hash != $buildMiss.output_hash),
      untraced: $untraced
    }
  }' >"$results/summary.json"

#!/usr/bin/env bash
set -euo pipefail

# CIP-108: source size distinguishes production coupling from colocated tests,
# while the total physical-file ceiling remains the guardrail.
limit=2000
large_test_module=500
failed=0

inline_test_lines() {
  local source=$1
  local total=$2
  local start

  # Rust's conventional inline test module is trailing. Keeping it trailing
  # gives this deliberately cheap source check an exact physical-line count
  # without pretending to parse Rust syntax in shell.
  start=$(awk '
    /^#[[:space:]]*\[cfg\(test\)\][[:space:]]*$/ {
      attribute = NR
      next
    }
    attribute {
      if ($0 ~ /^(pub[[:space:]]+)?mod[[:space:]]+tests[[:space:]]*\{/) {
        print attribute
        exit
      }
      if ($0 !~ /^[[:space:]]*$/) {
        attribute = 0
      }
    }
  ' "$source")

  if [ -n "$start" ]; then
    printf '%s\n' "$((total - start + 1))"
  else
    printf '0\n'
  fi
}

check_module_map() {
  local root=$1
  local declared map_block mapped missing stale malformed

  declared=$(sed -nE \
    's/^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?mod[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*[;{].*/\3/p' \
    "$root" | sort -u)
  if [ -z "$declared" ] || [ "$(printf '%s\n' "$declared" | wc -l)" -lt 2 ]; then
    if ! grep -q '^//! Intentional module-map omission:' "$root"; then
      printf '%s\n' "ERROR $root has fewer than two direct modules but no explicit intentional module-map omission."
      failed=1
    else
      printf '%s\n' "MODULE MAP OMITTED $root: explicitly intentional for fewer than two direct modules."
    fi
    return
  fi

  if ! grep -q '^//! ## Module map$' "$root"; then
    printf '%s\n' "ERROR $root declares multiple direct modules but has no crate-root module map."
    failed=1
    return
  fi

  map_block=$(awk '
    /^\/\/! ## Module map$/ { in_map = 1; next }
    in_map && /^\/\/! - / { saw_entry = 1 }
    in_map && saw_entry && /^\/\/!$/ { exit }
    in_map { print }
  ' "$root")
  malformed=$(printf '%s\n' "$map_block" \
    | grep '^//! - ' \
    | grep -vE '^//! - `[A-Za-z_][A-Za-z0-9_]*`: .+\.$' || true)
  if [ -n "$malformed" ]; then
    printf '%s\n' "ERROR $root has malformed module-map entries; each needs one ownership sentence."
    failed=1
  fi

  mapped=$(printf '%s\n' "$map_block" \
    | sed -nE 's@^//! - `([A-Za-z_][A-Za-z0-9_]*)`: .+\.$@\1@p' \
    | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$declared") <(printf '%s\n' "$mapped"))
  stale=$(comm -13 <(printf '%s\n' "$declared") <(printf '%s\n' "$mapped"))
  if [ -n "$missing" ] || [ -n "$stale" ]; then
    [ -z "$missing" ] || printf '%s\n' "ERROR $root module map is missing: $(printf '%s' "$missing" | tr '\n' ' ')"
    [ -z "$stale" ] || printf '%s\n' "ERROR $root module map has no declaration for: $(printf '%s' "$stale" | tr '\n' ' ')"
    failed=1
  else
    printf '%s\n' "MODULE MAP $root: $(printf '%s' "$declared" | tr '\n' ' ')"
  fi
}

while IFS= read -r root; do
  check_module_map "$root"
done < <(find crates -path '*/src/lib.rs' -o -path '*/src/main.rs' | sort)

while IFS= read -r source; do
  total=$(wc -l < "$source")
  inline_tests=$(inline_test_lines "$source" "$total")
  live=$((total - inline_tests))
  printf '%s\n' "SOURCE $source: live=$live LOC inline-test=$inline_tests LOC total=$total LOC"

  if [ "$inline_tests" -ge "$large_test_module" ]; then
    printf '%s\n' "TEST-MODULE EXTRACTION $source: $inline_tests inline-test LOC; consider moving the test module beside this source."
  fi

  if [ "$total" -le "$limit" ]; then
    continue
  fi
  case "$source" in
    crates/cix-build/src/build_chain.rs)
      printf '%s\n' "GRANDFATHERED $source: live=$live LOC inline-test=$inline_tests LOC total=$total LOC; Workspace ownership is extracted, while memo/context/sandbox/FETCH-state legs remain; see cix-build/src/lib.rs module map."
      ;;
    *)
      if [ "$live" -gt "$limit" ]; then
        printf '%s\n' "ERROR $source has $live live LOC (total=$total LOC; limit $limit); split the production stratum and update its crate-root module map."
      else
        printf '%s\n' "ERROR $source has $total total LOC (limit $limit) because of $inline_tests inline-test LOC; extract the test module."
      fi
      failed=1
      ;;
  esac
done < <(find crates -path '*/src/*.rs' -type f | sort)

exit "$failed"

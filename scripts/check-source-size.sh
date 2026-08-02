#!/usr/bin/env bash
set -euo pipefail

# CIP-89: the crate-root module maps explain how to split a new stratum.
limit=2000
failed=0
while IFS= read -r source; do
  lines=$(wc -l < "$source")
  if [ "$lines" -le "$limit" ]; then
    continue
  fi
  case "$source" in
    crates/cix-build/src/build_chain.rs)
      printf '%s\n' "GRANDFATHERED $source ($lines LOC): the conductor still contains workspace and memo helpers; see cix-build/src/lib.rs module map."
      ;;
    *)
      printf '%s\n' "ERROR $source has $lines LOC (limit $limit); split the new stratum and update its crate-root module map."
      failed=1
      ;;
  esac
done < <(find crates -path '*/src/*.rs' -type f | sort)
exit "$failed"

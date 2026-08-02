# Thinning round — measured hotspots + alpha-compat deletions

Status: **draft** (2026-08-02, drive-progress step-2 sweep; complexity-
monster method: measure, decompose along strata, delete speculative
compat in alpha).

## 1. The problem — measured

Today's eight merged tracks concentrated their mass in three files:

| file | LOC | fns | churn (2 days) |
| --- | ---: | ---: | ---: |
| cix-build/src/build_chain.rs | 3492 | 115 | absorbed readset+overlay+secrets in ONE day |
| cix-run/src/unit.rs | 2125 | 61 | 11 commits — every track touches it |
| cix-compose/src/{model,resolve,generation}.rs | 651+1277+1371 | — | tree1 alone grew model.rs by ~650 lines |

`build_chain.rs` now holds at least five distinct strata: step
execution/sandboxing, FETCH networking + the credential/consent store,
trace-capture glue, memo/replay, and workspace/underlay management.
Three concurrent tracks produced a seven-conflict merge in it — the
merge pain IS the measurement.

## 2. Prior work (house)

The split pattern already exists and works: `trace.rs` (683 LOC,
readset) and `closed_root.rs` (227 LOC, CIP-84) both landed as
separate modules against the grain of "append to the big file".
Complexity-monster memory: thin hotspots proactively, before they make
every future track's merge expensive.

## 3. Recommendation

One thinning track, no behavior change, snapshot-tests as the proof:

1. **build_chain.rs → modules along the strata**: `fetch.rs`
   (network + credentials/consent), `memo.rs` (constructive-trace
   memo/replay), `workspace.rs` (underlay lifecycle), keeping
   `build_chain.rs` as the thin conductor.
2. **unit.rs**: continue the closed_root.rs pattern — extract the
   per-feature property assemblers (devices, health, dirs
   materialization) into modules; unit.rs keeps ordering authority
   (property ORDER is contract — pin it with the existing fixtures).
3. **compose crate strata pass** after netns lands (model/resolve/
   generation each absorbed a tree; wait for the last one).
4. **Alpha-compat deletion audit** (D72: schema moves freely, alpha
   owes no compat): cix-index/src/refs.rs carries 23 legacy/compat
   markers, cix-build/src/lock.rs 13; README still claims "manifest
   v5; reads v1–v5" while fixtures are all `cixManifest:0` —
   inventory every version-reader/legacy branch, delete what nothing
   reachable uses, fix the README claim to the truth. Refusal-with-
   teaching-error stays (that is UX, not compat); silent old-format
   ACCEPTANCE goes.

## 4. Open questions

1. Timing: one track now, or hold until netns lands so the compose
   pass rides along? (Draft leans: build_chain+unit.rs now — they are
   stable; compose pass as a second leg.)
2. Does the manifest version story deserve one line of decision
   (e.g. "alpha reads exactly version 0") recorded as a D72 note?

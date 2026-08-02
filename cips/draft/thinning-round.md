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

## 4. The 4× turn-over (requested; each turn amends §3)

**Turn 1 — "file splits are cosmetic; the coupling stays."** Moving
115 functions into four files that all mutate one build-context
struct reproduces the tangle with extra imports. *Survives as:* the
split is defined by OWNED TYPES with narrow interfaces
(`ConsentStore`, `MemoStore`, `Workspace`), the conductor composes
them; a stratum that cannot get a narrow interface has revealed real
coupling — that finding is surfaced, not papered over. Acceptance is
each module's contract stated in a doc comment, not a line count.

**Turn 2 — "refactor-under-parallel-tracks is a merge bomb."** Today's
seven-conflict merge is the proof; reorganizing a file while a track
flies makes every future fix round conflict against moved code.
*Survives as:* hard precondition — thinning runs only when NO track is
in flight in that crate; pure-move commits (rename-detectable)
strictly separated from any behavior change. This also answers the old
timing question: cix-build and cix-run are quiet now (netns lives in
cix-compose), the compose pass waits for netns.

**Turn 3 — "deleting compat can delete correctness."** Alpha owes no
compat to external users, but PERSISTED artifacts are real: index
state on live hosts, committed corpus/example locks, warm memos. A
legacy reader may be load-bearing for state that exists. *Survives
as:* the audit classifies every compat branch by which persisted
artifact still exercises it (grep the committed locks; check the live
index format), deletes only what provably nothing reachable uses, and
where old state blocks deletion prefers regenerate-in-alpha
(wipe/fingerprint-bump precedent) over keeping the reader.

**Turn 4 — "thinning without a growth rule is a treadmill."** Agents
append to big files because it is the path of least resistance; the
next six tracks would re-fatten build_chain. *Survives as:* the module
map is recorded in the crate (doc comment) + one AGENTS.md line ("new
feature strata get new modules"), and a cheap warn-tripwire in the
gate: any src file crossing 2000 LOC fails with a pointer to the map —
crude, but it converts silent regrowth into a visible decision.

## 5. Open questions

1. Tripwire threshold (2000 LOC?) and whether it hard-fails or warns.
2. Does the manifest version story deserve one line of decision
   (e.g. "alpha reads exactly version 0") recorded as a D72 note?

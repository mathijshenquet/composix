# track/thin1 — CIP-89 leg 1: build_chain/unit strata + compat audit

Read AGENTS.md first (focused agent gate; synchronous receipts).
Authoritative: cips/accepted/0089-thinning-round.md — §3 as amended by
ALL FOUR §4 turns; they are the spec's law. Work in
`/home/mathijs/worktrees/composix/track-thin1` (herdr worktree) on
branch `track/thin1`. Keep `crates/cix-build/LOG.md`... that file may
not exist — use `crates/cix-cixfile/LOG.md` (the builder's journal).
QUIET-CRATE PRECONDITION (turn 2): track/netns runs in cix-compose —
you own cix-build, cix-run, cix-index; touch NOTHING in cix-compose.

1. **build_chain.rs → owned-type modules** (turn 1): extract along
   the strata into modules each owning a type with a narrow,
   doc-commented contract — suggested: `fetch.rs` (network +
   ConsentStore), `memo.rs` (constructive-trace memo/replay),
   `workspace.rs` (underlay lifecycle); build_chain.rs remains the
   conductor. PURE-MOVE COMMITS: every move commit is behavior-free
   and rename-detectable; interface-introduction commits are separate
   and minimal. A stratum that resists a narrow interface: STOP and
   record the coupling finding in the LOG rather than forcing it.
2. **unit.rs per-feature assemblers**: continue the closed_root.rs
   pattern — devices, health, and dirs-materialization property
   assembly move to modules; unit.rs keeps ordering authority.
   Property ORDER is contract: the existing unit fixtures must be
   byte-identical after every commit (that is the pure-move proof).
3. **Growth tripwire** (turn 4): a gate check failing any
   crates/**/src file over 2000 LOC, message pointing at the module
   map (record the map as doc comments in each thinned crate root).
   Add the one-line AGENTS.md rule ("new feature strata get new
   modules"). Grandfathered files that remain over the line after
   this leg (if any) are listed IN the check with a reason — visible,
   never silent.
4. **Alpha-compat audit** (turn 3) over cix-build/src/lock.rs and
   cix-index/src/refs.rs: classify every legacy/compat branch by the
   persisted artifact that still exercises it (committed corpus/
   example locks, live index state formats); delete what provably
   nothing reachable uses (evidence in the LOG per deletion); where
   old persisted state blocks, prefer regenerate-in-alpha
   (fingerprint-bump precedent) and do it. Produce the truthful
   manifest-version line (what does spec.rs actually accept?) —
   correct README.md's "reads v1–v5" claim to the proven truth and
   record it as a D72 note in the CIP changelog.
5. Docs: module maps; AGENTS.md line; CIP-89 changelog "leg 1
   landed". The compose-crate pass is leg 2 (after netns) — do not
   start it.

Gate (agent side): fmt / examples fmt / warning-denied clippy / full
workspace tests (byte-identical fixtures are the refactor proof) /
tour regen + drift / focused: vm-dogfood only (no scenario semantics
change). Full matrix at the orchestrator gate. Commit on this branch
when green.

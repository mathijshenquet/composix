# track/thin2 — CIP-89 leg 2: the compose-crate strata pass

Read AGENTS.md first (focused agent gate; synchronous receipts).
Authoritative: cips/accepted/0089-thinning-round.md §3 as amended by
the §4 turns — owned-type splits, pure-move commits, quiet-crate
precondition (satisfied: nothing else runs in cix-compose), growth
tripwire already in CI. Work in
`/home/mathijs/worktrees/composix/track-thin2` (herdr worktree) on
branch `track/thin2`. Keep `crates/cix-compose/LOG.md` current.
FENCE: track/fetchself runs concurrently in cix-build — do not touch
it.

1. cix-compose absorbed tree1 + netns + dirs2 + health in three days:
   model.rs/resolve.rs/generation.rs each carry multiple strata.
   Measure first (LOC, fn counts, cohesion by feature), then split
   along owned-type seams — candidates: netns/pod realization,
   directory materialization, health wiring, publish/socket
   machinery — with generation.rs as the conductor. Turn-1 rule: a
   stratum that resists a narrow interface is a recorded coupling
   finding, not a forced split.
2. Pure-move commits, rename-detectable, separated from any interface
   commit; all fixtures byte-identical after every commit (the
   refactor proof).
3. Module maps as crate-root doc comments; every file under the 2000
   tripwire or explicitly grandfathered in the check with a reason.
4. If lock.rs/schema shapes need touching, additive only.

Gate (agent side): fmt / examples fmt / clippy / workspace tests /
tour regen + drift / focused: scenario-tree + scenario-netns +
scenario-dirs2. Full matrix at the orchestrator gate. Commit when
green.

@AGENTS.md

Orchestrator notes (Claude only):
- Session start ritual: read `.dev/LOG.md` top entry, then `docs/design.md` "Building now" — that's sufficient context; explore deeper only on demand.
- Mathijs decision queue lives in the LOG's "Open with Mathijs" line — surface it at session start, don't re-derive it.
- Delegation: implementation work goes to codex agents exclusively (Claude stays orchestrator/designer/reviewer). Model pick: luna only for very rote work; terra when the spec is tight; sol when the task needs taste or decisions on the fly.
- Micro-fix exception (Mathijs, 2026-07-30; generalized 2026-08-02): work may be orchestrator-direct whenever the expected delegation prompt would be longer than doing the fix itself, with some buffer. Rationale: delegation buys exactly two things — clean orchestrator context and output-token savings — and a prompt longer than the fix forfeits both. Typical cases remain merge seams, conflict resolution, CI/tour rounds, doc nits. The full agent gate (fmt, clippy, tests) applies to those commits unchanged.

## Goals (/goal shorthand)

Mathijs activates a goal with `/goal <name>` in chat; `/goal stop` (or a new
`/goal`) deactivates. One goal active at a time; session-bound unless the
activation says `standing`. A goal grants direction-autonomy, never
decision-autonomy: new design decisions stay joint, everything lands through
the normal track/gate/merge discipline, and every goal-driven launch is
announced in chat as it starts (Mathijs watches via /rc).

### drive-progress

1. **Open ends first**: any accepted CIP or other recorded open end that is
   not implemented, in-flight, or explicitly slotted with a reason →
   implement or schedule it. Blockers are surfaced, never silently queued.
   Agent slots stay filled within machine capacity (max 4 concurrent).
2. **Dry → prospect once**: when no open ends remain, do ONE codebase/design
   sweep for possible unblockers — cleanup and quality inventories count —
   and land the findings as `cips/draft/` entries for adoption, each linked
   to Mathijs by full GitHub URL (the standing CIP-README rule). Drafts are
   the only output of this step: prospecting never starts implementation on
   its own authority. (One ontology: everything prospective is a CIP draft.)

Idle-state: when step 1 is empty and step 2's sweep has already run this
activation, the correct action is nothing — say so and wait.

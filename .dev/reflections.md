# Session reflections

Close-of-session ritual entries: model evaluation, project progress,
risks/opportunities, process reflection. Newest first. The model table
itself lives in nix-config (global.CLAUDE.md); these entries carry the
narrative behind its deltas.

## 2026-08-05 — the marathon (25 merges; corpus 21 -> 28; CIP-96..109)

### Models

- **terra** (~15 tracks): implementation uniformly strong, including
  the hard fixes (mountpoints-in-artifact, Workspace/Memo extractions,
  lock aggregation, sweep liveness guard). The failure taxonomy was
  almost entirely process, clustered EARLY: three detached-gate
  claims, one false green on its own scenario, one ignored merge
  instruction, chronic uncommitted turn-ends — and it largely vanished
  once capture-as-epilogue + value-checked receipts entered the specs.
  Conclusion: terra executes exactly the receipts discipline you write
  down; it brings none of its own. Late-day highlights were exemplary
  honesty: lockagg restoring a non-identical experimental re-lock,
  cip107's evidence-based non-deletion.
- **sol** (4 taste-heavy tracks): zero overclaims. Structural audit
  with verified numbers + six adoptable drafts in ~20 minutes; the
  contract-keyed selector with a measured 98.2% win and argued
  rejections; k8s teaching contract; MemoEngine narrow interface. Only
  intervention: trimming over-verification — sol re-earns receipts
  unprompted; cap the scope, never the honesty.
- **luna** (11 cold stagings + experiments): middling band handled
  (source compiles, gpg dances) green-or-precisely-walled; FRICTION
  journals paid out same-day (probe-url: stumble -> draft -> adopted
  -> implemented within hours). One staging claim disproved at
  assembly (adminer cold wall) — independent assembly re-verification
  stays load-bearing.
- **Orchestrator (fable)**: the gate layer caught four real defects
  and the discriminate-before-accept protocol went 4/4. But personal
  process errors scaled with the fleet: a grep-line-presence merge
  (one ungated merge reached main), an unconditional cleanup chained
  past a failed value-check (swept two live worktrees), a heartbeat
  lapse, cached-phase serialization twice, silently discarded prompt
  refusals — five of these caught by Mathijs, not me. Every one was
  canonized into CLAUDE.md the same day; the pattern is that 4-slot
  process rules broke at 8+ slots.

### Progress

Corpus 21 -> 28 with receipts and a meaningful ribbon vocabulary;
CIP-96..109 implemented except 103's context/FETCH legs and 104;
build_chain 4,369 -> 2,059 live; runtime.rs 1,969 -> 290; VM tier
98.2% cheaper for build-subsystem diffs; -376k lock lines; the k8s
axis opened; the language epoch fully designed and awaiting adoption.

### Risks

1. Epoch delay compounds: the nodes-and-edges/phase-blocks sweep grows
   with every expansion wave, and fmt-key-neutrality MUST land first
   (a reformatting sweep under a key-changing fmt would churn every
   lock).
2. The pnpm wall (five exhibits) sits exactly where real-world
   Dockerfiles live.
3. Aggressive-hygiene-vs-live-processes is a CLASS (sweep liveness was
   one instance; lock GC and similar deserve the same look).
4. Single-host fleet: three contention axes, two disk incidents in one
   day; CI-runner-vs-beast environment deltas keep producing
   env-class failures.

### Opportunities

1. The epoch as ONE sweep — wave machinery proved ~7 cases/day.
2. Selector dividend: per-track wall-clock collapsed; more tracks/day
   is structurally affordable.
3. k8s wave 1 is one adoption call away.
4. pnpm-wall + the WITH CACHE direction could crack the dominant
   real-world blocker in one design round.

### Process

Worked: watchers over polling; discriminate-before-accept (4/4);
value-checked receipts; friction journals; delegated review with KPIs
(six CIPs adopted and landed same-day). Canonized after failing:
value-checks on captures, capture-as-epilogue (never pre-touch,
empty != receipt), watcher-at-launch, re-arm-on-every-wake,
queue-walks against real gate conditions, the codex delivery matrix,
gate composition (cargo tier ownership). Meta-lesson, Mathijs's call
vindicated same-day: canon in versioned files beats memory — later-day
me applied the recipes from CLAUDE.md, not from recall.

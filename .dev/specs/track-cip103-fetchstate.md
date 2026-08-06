# track/cip103-fetchstate — CIP-103 legs 4+5 (context/sandbox boundaries, FETCH-state owner)

Read cips/accepted/0103-build-chain-seams.md first — this track executes
its remaining legs 4 and 5 (legs 1–3, Workspace and MemoEngine owners,
already landed; build_chain.rs is at 2059 lines live).

Leg 4: introduce request-result boundaries around Nix evaluation for
context/sandbox — the conductor asks a question and receives an answer;
no bag-of-fields shared context object crosses the seam (the CIP's
explicit anti-goal).

Leg 5: move FETCH snapshot volatility and pin refresh behind a
FETCH-state owner. `build_chain` keeps only ordered FETCH/BUILDER
dispatch and receipt assembly.

KPIs (same as the prior legs): build_chain.rs line count down
substantially; seams clean (each owner has a narrow, nameable
interface); no behavior change — the full test suite and tour must be
byte-identical green. Follow the ownership style of the landed
Workspace/MemoEngine extractions (look at their modules before
designing). Interior mutability requires a stated justification
comment per repo convention; expect none to be needed.

Discipline: branch `track/cip103-fetchstate` from current main, LOG
`crates/cix-build/LOG.md`; full agent tier (fmt / examples fmt /
clippy -D warnings / full workspace tests / tour regen+drift +
progressive VM check); value-checked synchronous captures only —
`cmd; echo $? > .gate-exit` epilogue, never detached output as
receipt. Keep a FRICTION section in your LOG: anything not
immediately intuitive about the codebase or task. Clean branch; do
not merge.

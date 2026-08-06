# track/epoch-stage2 — the epoch's execution semantics (CIP-111/112/113 stage 2)

Charter: the Decision chapters of `cips/accepted/0111-nodes-and-edges.md`,
`0112-phase-blocks.md`, `0113-build-args.md`, plus the STAGE-2 HANDOFF
terra recorded in `crates/cix/LOG.md` (search "Stage 2" around lines
110–140: exact remaining semantics per layer, the seams a fresh
implementer must touch). Stage 1 (parser/fmt/AST, dual acceptance) is
merged on main; `track/fmtkey-impl` (canonical-key API per CIP-110) may
merge while you work — coordinate through main, merge it in when it
lands.

Scope — the semantics stage:

1. **Execution**: argv-first RUN/FETCH through the sandbox boundary
   (direct exec, no shell); mandatory-interpreter heredocs (body to
   file, interpreter invoked with the filename); WITH env edges bound
   per node (assignment + bare LET-bridge forms); LET juxtaposition
   word-list expansion into argv elements.
2. **`WITH UNSAFE IGNORE <path>`**: evidence exclusion at the
   trace/seal/key seams + the use-site diagnostic naming the waived
   evidence; plus the conservative detection/surfacing diagnostic
   (propose-the-clause hints per CIP-114 §4's settled design —
   surface, never auto-classify).
3. **ARG CLI/lock/manifest**: `cix build --arg NAME=value` (undeclared
   → error listing the declared matrix), `--all-args`, per-cell
   append-only lock entries via resolved-statement keying, manifest
   records the selection.
4. **Keying** for all new constructs per CIP-111's edge-granularity
   section, through the canonical-serialization seam (fmtkey-impl's
   API once merged; a clearly-marked same-shape stub until then).
5. **Tests**: executor semantics (argv, heredoc file-contract, WITH
   env isolation, LET expansion, ARG selection), teaching errors
   verbatim with D73 doc anchors, UNSAFE-IGNORE evidence exclusion
   proven in a trace test.

The corpus sweep + migrate.md rewrite stay OUT (final epoch track);
old grammar keeps parsing (dual acceptance intact this stage).

KNOWN ENVIRONMENT LIMIT (recorded on stage 1): your terminal bridge
reaps detached processes — the .gate-exit recipe does NOT work in this
pane. Run what fits your foreground bound (fmt, examples fmt, clippy,
focused suites); the workspace + VM tiers are DELEGATED to the
orchestrator merge gate — state that explicitly in your LOG instead of
attempting them. Commit the clean branch on your valid receipts.

Discipline: branch `track/epoch-stage2`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). If a CIP decision proves
ambiguous in implementation, STOP on that item, record the exact
conflict, continue with the rest. Synchronous value-checked receipts
only. Clean committed branch; do not merge.

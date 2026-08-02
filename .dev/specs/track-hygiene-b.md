# track/hygiene-b — CIP-90 leg B: compose boundary, ps --json, tour from one truth

Read AGENTS.md first (focused agent gate; synchronous receipts).
Authoritative: cips/accepted/0090-test-hygiene.md (§3 + §5 + the
shared-state changelog amendment). Leg A landed the clap boundary
outside cix-compose. Work in
`/home/mathijs/worktrees/composix/track-hygiene-b` (herdr worktree) on
branch `track/hygiene-b`. Keep `crates/cix-compose/LOG.md` current.
FENCE: track/tracefast (a Claude agent) owns crates/cix-build and the
COPY/staging/trace machinery — touch NOTHING there. Your domain:
cix-compose, the cix CLI crate, crates/cix/tests/tour.rs.

1. Finish the compose env boundary: sweep remaining `CIX_*` reads in
   cix-compose into the clap/config boundary; remove the compose
   allowlist entry from scripts/check-cli-env-boundary.sh.
2. `cix ps --json`: public machine output (docker --format json
   precedent) from the same data that renders the table.
3. Tour reads from one truth: harness assertions/filters consume
   ps --json; the human table renders from the same source — the
   width-drift class dies. Delete TOUR_RENDER_LOCK if the env/global
   reason for it is gone (verify what it guards first; if it guards
   something real and non-env, keep it WITH the now-required
   justification comment).
4. Tests: --json golden output; tour green; boundary lint green with
   the allowlist gone; grep receipts that no set_var remains anywhere.

Gate (agent side): fmt / examples fmt / clippy / workspace tests /
tour regen + drift / focused: scenario-observability + scenario-tree.
Full matrix at the orchestrator gate. Commit when green.

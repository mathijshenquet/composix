# track/pinkeys — D69 implementation: consumed-set keying, offline --cold, probe, codegen fingerprint

Read AGENTS.md first. Authoritative: design.md **D69 (a,b,c,e)** in full. Evidence
and mechanics: the diagnosis report at
/tmp/claude-1001/-home-mathijs-composix/9f54ba2f-dff1-4113-9551-0a8e2b7a9542/scratchpad/fetch-diagnosis-report.md
(read it; it has file:line for every current code path and the byte-level npm
diffs). Scope: crates/cix-cixfile (build_chain/lock), docs (cixfile.md boundary
paragraph, migrate.md fetch guidance touch-up), corpus re-checks as proof.

1. **Consumed-set keying (D69a)**: the automatic FETCH pin narrows from
   whole-tree narHash to a path→hash map over the paths downstream steps
   actually consume (extend the existing D57 consumed-output tracking to fetch
   outputs; the report cites where reads are tracked today). First-use of a
   previously unread path under an existing pin: record loudly as a fresh pin
   entry (stderr note naming the path), never silently. A declared EXPECT
   stays whole-tree and unchanged.
2. **Offline --cold (D69e)**: `--cold` replays RUN steps from empty
   workspaces but reuses pinned FETCH outputs of BOTH kinds — no network on
   any --cold path (test proves no fetch process spawns). Fetch re-execution
   happens only on `--update-lock` and memo-miss-without-pin.
3. **Update-lock double-fetch probe (D69b)**: on `--update-lock` touching a
   FETCH, run the fetch twice; diff; report volatile files loudly (names +
   sizes) and record the volatile set in the lock as fact. With (1), the pin
   itself is over consumed paths; the probe output is informational + feeds
   (c) normalization guidance.
4. **Codegen fingerprint in memo/chain keys**: a cix build-logic fingerprint
   (e.g. hash of the cix version + codegen-relevant build id) joins the pure
   key inputs so a stale checkout can never get memo hits from bytes built by
   different cix code (tonight's tour-narHash cross-pollution). Note in LOG
   what invalidation this causes (one-time global memo miss — expected).
5. Docs: the --cold boundary paragraph (what it proves, what the pin proves);
   dozzle-ui's unstable consumed `dist` stays a recorded honest fail (D69c
   posture: record; act only if the class recurs).
6. **Proof against the exhibits**: fetch contexts, then show parse-server and
   projB pins now stable across two clean `--update-lock` runs and a
   subsequent ordinary build (previously flapping); dozzle's go-side FETCH
   stable under consumed-keying; dozzle-ui pnpm recorded as the known
   normalization case. Update those receipts + corpus LOG honestly.
7. Gate: fmt / warning-denied clippy / workspace tests / cold_audit if
   present on this base / tour regen + drift + determinism twice / vm-dogfood
   / the exhibit proofs of point 6. Exact repros in crates/cix-cixfile/LOG.md.
   Commit on this branch when green.

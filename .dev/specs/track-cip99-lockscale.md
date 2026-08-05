# track/cip99-lockscale — lock scale per CIP-99

Read first: `cips/accepted/0099-lock-scale.md` — it is the decision
(subtree aggregation with the 4x coherence check); this spec adds
track mechanics only. Motivating evidence from today: parse-server's
lock diff in the regen3 merge was ±480k lines; echo-server/phpmyadmin
locks are node-tree-shaped monsters.

Implement exactly what the CIP decides. Where it leaves implementation
choices open, pick conventionally, record in LOG + CIP changelog;
genuine design questions it does not answer are STOP-and-flag.
Regenerate affected corpus locks as the acceptance exhibit (before/
after line counts per case in the LOG — the number is the deliverable);
byte-identical build outputs (same store paths) prove the aggregation
is representation-only. Ledger currency per AGENTS.md if evidence
classes change; browser regen if lock renders are shown.

Discipline: branch `track/cip99-lockscale`, LOG
`crates/cix-cixfile/LOG.md`; full agent gate tier, capture-as-epilogue,
bounded VM parallelism. Parallel tracks in flight (incl. cip93b's
selector) — merge semantically. Clean branch; do not merge.

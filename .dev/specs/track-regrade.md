# track/regrade — corpus & migrate re-analysis against the 2026-08-01/02 feature wave

Read AGENTS.md first. Context: since the corpus survey (2026-07-30) and
the migrate-prompt empirical round, a large feature wave landed: D47
blocks/binders, D74 fmt, CIP-75 timers (compose `schedule:`), CIP-76
`cix watch`, CIP-80 START/START_PRE, CIP-82 leg 1 dirs (claims model,
overlay backing, `DIR`), CIP-83 observability (`cix logs`/`stats`,
exit-cause mapping). The ledgers still grade against the old feature
set and should be much greener. Your job: re-analyze honestly, with
receipts where cheap. Work in `.worktrees/regrade` on branch
`track/regrade`. Keep `crates/cix-cixfile/LOG.md` current.

1. **Sweep docs/corpus.md**: every ribbon whose blocker cites a
   now-landed feature gets re-graded. The honesty caveat stands: a
   grade only upgrades to ✅-with-receipt if you actually converted and
   ran it this round; otherwise it stays a desk grade, re-worded to
   cite the landed mechanism. Mark each updated row's evidence class
   explicitly (desk / receipt).
2. **Empirical subset**: pick the rows where a landed feature flips
   the grade (expect: timer/cron shapes → CIP-75; volume/bind-mount
   blockers → CIP-82; healthcheck rows stay ⏳ CIP-79-queued; logging
   rows → CIP-83). Convert per docs/migrate.md, build, and where a VM
   or local run is cheap, run + probe. Receipts (exact commands,
   pass/fail) in the LOG. Time-box: this is a re-analysis, not a
   51-app port — prioritize grade-flipping rows; list what you did NOT
   re-verify.
3. **docs/migrate.md**: the gap table gets the same sweep — rows whose
   workaround text predates dirs/obs/timers/start get rewritten against
   today's directives (DIR/CLAIM/START/SCHEDULE spellings). Both doc
   samples must still independently re-build green if you touch them.
4. **docs/docker.md**: sweep for rows that the dirs/obs tracks missed
   or that new landings make stale; fix honestly.
5. **FENCE — track/devices is in flight and owns**: corpus rows 7
   (Immich) and 17 (Frigate) mechanism columns, and docker.md's
   `--device`/`--gpus`/`--shm-size`/`--group-add`/tmpfs rows. Do NOT
   edit those; where your sweep would touch them, note "pending CIP-78
   impl (in flight)" in your LOG instead.
6. **Corpus §4 demands + §5 example candidates**: update the demand
   ranking honestly (which demands are now met, which remain), and
   re-evaluate the §5 candidates against today's directive set.
7. New convention (AGENTS.md): ledgers stay current from now on —
   future tracks re-grade affected rows themselves. Your job is the
   one-time catch-up; note in docs/corpus.md's status header that the
   sweep date is 2026-08-02 and grading is maintained per-track from
   now on.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.

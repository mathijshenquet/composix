# track/mastodon — the corpus §5 integration flagship (post-79/82)

Read AGENTS.md first (focused agent gate; synchronous receipts;
authoring canon: real files + COPY, no &&-chains, heredoc only with
interpolation + comment). docs/corpus.md §5 names the Mastodon-shaped
stack the top integration candidate once CIP-79 (health) and CIP-82
(dirs incl. shared surfaces) landed — both landed today, plus secrets
(CIP-81) and observability (CIP-83). This track is the integration
proof that the year's features compose. Work in
`/home/mathijs/worktrees/composix/track-mastodon` (herdr worktree) on
branch `track/mastodon`. Keep `crates/cix-cixfile/LOG.md` current.
PARALLEL FENCE: track/netns (cix-compose code) and track/hygiene-a
(cix-run/build/index code) run concurrently — this track writes NO
product code: corpus/example/doc files only. If you hit a product bug,
record it in the LOG and report it as a finding — do not fix product
crates. Use HOST networking (`network:` is unreleased; the netns
track owns it).

1. **The stack** (corpus compose row 9, behaviorally faithful at the
   composition level): web (puma), sidekiq worker, streaming, postgres,
   redis — Cixfiles per member (overlay universes where package
   composition demands, D70), one compose tree using today's features
   honestly: shared-rw `shared:` surface between web and sidekiq
   (CIP-82's headline case), READINESS/LIVENESS probes (CIP-79),
   SECRET delivery for the DB password (CIP-81), unix edges where the
   upstream uses localhost, `schedule:` for the cleanup cron
   (CIP-75). `CLAIM egress` on exactly the members that reach out.
2. **Honesty rules**: what cannot be expressed yet (`internal:true`
   segmentation → D26/D27 era; anything else you hit) is recorded in
   the receipt as a loss, not worked around silently. If full Mastodon
   proves too heavy to RUN in the check (asset compilation, boot
   cost), the pre-agreed fallback is a faithfully-shaped stack
   (real postgres/redis/nginx + a puma-shaped app stub) — record the
   choice and why; behavioral shape over brand-name completeness.
3. **Receipts**: `corpus/migrate/mastodon/` per the house layout
   (SOURCE, check.sh, receipt.md); `./check.sh cix` passes
   synchronously — the check proves: both web and sidekiq write the
   shared surface; readiness gates `cix up`; the secret arrives via
   `$CREDENTIALS_DIRECTORY`; the scheduled job fires; `cix logs`
   selects a member.
4. **Ledgers**: corpus rows 9/10 re-graded with the receipt (the
   shared-rw gap row closes); §5 candidate note updated; the corpus
   browser regenerates (generator test).

Gate (agent side): fmt / examples fmt / warning-denied clippy /
workspace tests / tour regen + drift / focused: the mastodon check.sh
+ corpus generator drift. Full matrix at the orchestrator gate.
Commit on this branch when green.

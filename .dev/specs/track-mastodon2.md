# track/mastodon2 — mastodon members onto the CIP-91/92 canon

Read AGENTS.md first, then docs/corpus.md §"How this corpus is
maintained". Work in the herdr worktree on branch `track/mastodon2`.
Keep `corpus/migrate/LOG.md` current (dated heading; commit it). The
case lives at `corpus/migrate/docker/mastodon/` (post-restructure
path — verify; if the restructure has not merged yet, STOP and tell
the orchestrator).

The six member Cixfiles (postgres, redis, web, sidekiq, streaming,
cleanup) predate CIP-91: LINK piles, interpolated argv, pre-canon
style. This track modernizes them IN PLACE (not cold regen — the
compose flagship's semantics are proven by receipts and must not
drift):

1. Per member: LINK → IMPORT/COPY per canon; bare argv; role-dir paths
   reviewed against CIP-91 §3(d); no behavioral change — the
   compose.json contract, tags, ports, edges, secrets, readiness are
   untouched.
2. Rebuild every member; re-run the case `check.sh` end to end
   (the full six-member compose probe) with synchronous receipts.
3. GAPS.md refreshed (Generated header: this is a canon modernization,
   not a cold regen — say `modernized in place · <model> · date` on the
   Status line rather than pretending coldness); the CIP-91 stale flag
   resolves.
4. docs/corpus.md mastodon row re-graded to what the fresh receipt
   proves; browser regenerated.

FENCE: only the mastodon case directory, docs/corpus.md row, generated
browser, your LOG. If a member's modernization hits a real product wall
(the netns/pod surface, secrets), STOP and report rather than
improvising.

## Gate

Standard agent tier + the focused closed-root audit scenario (mastodon
is in its roster). df-guard; bounded. Synchronous receipts.

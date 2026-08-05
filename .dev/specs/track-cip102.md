# track/cip102 — volatile-fetch: diagnostic, teaching, EXPECT corpus sweep

Read first: `cips/accepted/0102-volatile-fetch.md` (the decision — small
by design; do not re-engineer), `docs/migrate.md`, `docs/corpus.md`
("How this corpus is maintained"), `AGENTS.md`.

## Scope (the CIP's three parts, verbatim)

1. **Diagnostic**: on EXPECT mismatch cix already names both hashes;
   add one teaching line — if a refetch of unchanged upstream diverges,
   the fetched tree is volatile: drop EXPECT and rely on TOFU consumed
   pins (show the command), or pin a stable asset URL instead.
2. **Teaching (migrate.md, one paragraph)**: EXPECT only what is
   stable (release tarballs, tagged files). For volatile or
   ecosystem-managed fetches use TOFU consumed pins, and when author
   trust is wanted verify upstream's published checksum inside RUN
   (`sha256sum -c` against the vendor value).
3. **Corpus sweep** (the wrongly-placed EXPECTs; keep every RUN-level
   upstream checksum verification):
   - `traefik` — remove the volatile release-JSON EXPECTs and record
     the two correct pins its corpus.md row promises ("stale by
     design … until the later corpus round normalizes the volatile
     metadata fetch and records the two correct pins" — this track IS
     that round). Rerun `check.sh` for the receipt.
   - `phpmyadmin` — the mirror-pipeline EXPECT (its known
     EXPECT-refetch mismatch); GPG + sha256 RUN verification stays.
   - `adminer` — the cold-unstable source FETCH EXPECT (receipt notes
     the unchanged mismatch); the `sha256sum -c` guards stay.
   - `echo-server` — audit its script-driven `FETCH bash
     install-dependencies.sh` EXPECT (a cold read-set divergence
     case): volatile → remove per the rule; genuinely stable → keep
     and say so in the receipt.
   After the sweep: update each case's GAPS.md and receipt honestly
   (what was rerun, what is now cold-stable vs still divergent),
   re-grade the affected docs/corpus.md rows (traefik's ⏳ especially),
   and regenerate the corpus browser (`cargo test --test corpus --
   --ignored generate_corpus_browser`).

Out of scope: regenerating whole cases (wave-3 handles caddy/
parse-server/directus/watchtower); ENV grammar (parallel track).
Expect parallel tracks on main — resolve merges semantically yourself.

## Discipline

- Branch `track/cip102`, this worktree. Log: `corpus/migrate/LOG.md`,
  timestamped, append-only.
- Gates (synchronous exit-status receipts, exact commands in the LOG):
  `cargo fmt --all --check`, `cix fmt --check examples`, warning-denied
  clippy, full workspace tests, tour regen + drift check,
  `devenv shell -- nix run .#progressive-vm-check` for what your diff
  selects. Corpus receipts: rerun `check.sh` per touched case,
  synchronous.
- df before big fetches: `df -h /` and `df -i /tmp`; clean your scratch.
- Commit granularly; leave the branch clean. Do not merge to main.

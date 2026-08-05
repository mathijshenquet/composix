# track/cip98 — artifact-root collision (role dirs anywhere)

Read first: `cips/accepted/0098-artifact-root-collision.md` — it is
the decision and the complete scope; this spec only adds track
mechanics. Context: role dirs under the application tree can collide
with the artifact's own mount (wallos was forced from `/var/www` to
`/app`; docker-volume nesting is the same shape).

Implement exactly what the CIP decides. Where it leaves an
implementation choice open, pick the conventional answer and record it
in the LOG and CIP changelog; genuine design questions it does not
answer are STOP-and-flag, not invent.

Ledger currency: after landing, grep `corpus/migrate/{docker,k8s}/*/GAPS.md`
for artifact-root/CIP-98 exhibiting cases (wallos) and flip them to
`Status: stale — regenerate with CIP-98`; docs/corpus.md's wallos row
currently carries 🔶⌛ for exactly this — its ⌛ becomes 🔄 when you
land (the fix moves from adopted to implemented; regeneration is then
the remaining step).

## Discipline

- Branch `track/cip98`, this worktree. Log: `crates/cix-run/LOG.md`
  or `crates/cix-build/LOG.md` per where the change lands.
- Gates (synchronous exit-0 receipts, exact commands in the LOG):
  `cargo fmt --all --check`, `cix fmt --check examples`, warning-denied
  clippy, full workspace tests, tour regen + drift,
  `devenv shell -- nix run .#progressive-vm-check` for what the diff
  selects; add a scenario assertion for a role dir inside the artifact
  tree.
- Parallel tracks are in flight on main (incl. a corpus assembly track
  touching docs/corpus.md) — resolve merges semantically yourself; your
  wallos row/GAPS edits may need re-application on top of it.
- Commit granularly; leave the branch clean. Do not merge to main.

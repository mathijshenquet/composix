# track/cip97 — granular hardening degradation

Read first: `cips/accepted/0097-granular-degradation.md` — it is the
decision and the complete scope; this spec only adds track mechanics.
Context: one rejected systemd directive currently drops a whole
hardening set (this broke the tour on GitHub CI's older user manager;
CI portability across the manager matrix is the requirement).

Implement exactly what the CIP decides (batched systemd-analyze verify
probing, per-directive granularity). Where the CIP leaves an
implementation choice open, pick the conventional answer, record it in
the LOG and the CIP changelog; if a genuine design question surfaces
that the CIP does not answer, STOP on that part and flag it in your
report rather than inventing semantics.

## Discipline

- Branch `track/cip97`, this worktree. Log: `crates/cix-run/LOG.md`,
  timestamped, append-only.
- Gates (synchronous exit-0 receipts, exact commands in the LOG):
  `cargo fmt --all --check`, `cix fmt --check examples`, warning-denied
  clippy, full workspace tests, tour regen + drift,
  `devenv shell -- nix run .#progressive-vm-check` for what the diff
  selects. Degradation behavior needs a scenario assertion (probe
  rejects one directive → only that directive drops, loudly).
- Parallel tracks are in flight on main — resolve merges semantically
  yourself. Serialize your tour/user-manager test runs if you observe
  foreign cix-* units (shared-manager races are a known false-red).
- Commit granularly; leave the branch clean. Do not merge to main.

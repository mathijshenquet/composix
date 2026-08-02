# track/ergo fix round — semantic merge seam after main caught up

Your track was built before devfix, health, and dirs2 landed; the
pre-gate main-merge (942b8cb) merged textually but the independent gate
found two semantic regressions via tour drift (chapter 3). Receipts:
SYNCHRONOUS exit statuses only (see the new AGENTS.md convention).

1. **Lock `outputs` section lost**: the committed tour's
   `prebuilt/Cixfile.lock` has an `outputs` map (per-artifact
   sourceHash + storePath — landed on main while you worked); your
   branch's generated lock omits it. Reconcile your lock
   writer/reader with the landed shape — additive, no format forks.
2. **`cix ps` empty after `cix run --detach --user`**: the committed
   tour shows the active unit row (devfix landed a user-manager
   PrivateDevices capability probe + degradation for exactly this).
   On your branch the row is gone again. Suspects: your no-op-floor
   subprocess elimination or in-process system detection interacting
   with the probe/degradation path (CIX_PRIVATE_DEVICES_PROBE,
   HostCapabilities), or the unit failing for a new reason. Diagnose
   with evidence (journal of the failed unit / probe result), fix
   without weakening either feature, and state the mechanism in the
   LOG.
3. Regenerate the tour; `git diff --exit-code -- docs/tour` must pass
   against the committed docs (the committed content is the target —
   it reflects main's landed behavior).

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Commit on this branch when
green.

# track/cip101 — tmp-relocate: scratch cleanup on every exit, big trees off tmpfs

Read first: `cips/accepted/0101-tmp-relocate.md` (the decision),
`AGENTS.md` (gates, receipts).

Motivating incident (today, again): host /tmp tmpfs reached 54% inode
use from composix scratch alone — observed patterns and inode counts:
`cix-step-delta-*` (312k inodes), `cix-build-view-*` (143k),
`composix-nixcompare.*` (30k), `composix-systemd-src.*` (8k), plus the
CIP-recorded `cix-fetch-probe-*` / `cix-build-cold-*` classes. Node
trees = tens of thousands of tiny files per directory.

## Scope (the CIP's three parts)

1. **Cleanup on every exit path is the primary fix**: every scratch
   tree cix creates (enumerate them — grep the workspace for tempdir
   creation; the patterns above are the observed leakers) is removed
   on success, failure, and signal (nix-style). A `--keep-scratch`
   flag opts into retention for debugging and prints the kept path.
   Prefer RAII/tempfile-crate ownership over ad-hoc rm calls; where a
   tree must outlive a process, name the owner responsible for it.
2. **Destination follows the systemd file-hierarchy guidance**:
   big-tree scratch (build views, step deltas, probe unpacks) moves
   off /tmp to disk-backed storage — pick `/var/tmp/cix-…` or
   `$XDG_CACHE_HOME/cix/tmp` per the CIP (your call; record the
   rationale in the LOG and CIP changelog), always honoring an
   explicit `$TMPDIR` override. Small short-lived files may stay on
   /tmp.
3. **Startup orphan sweep**: on cix startup, remove own scratch (the
   enumerated patterns, own uid) older than one day — belt and braces
   under tmpfiles aging.

Also: test-suite scratch (`composix-nixcompare.*`,
`composix-systemd-src.*` come from tests/tooling) gets the same
treatment where reachable; if some pattern belongs to test harness
code, fixing the harness is in scope.

Out of scope: /tmp debris of other tools (fleetd, node caches).

## Discipline

- Branch `track/cip101`, this worktree. Log: `crates/cix-build/LOG.md`,
  timestamped, append-only.
- Gates (synchronous exit-0 receipts, exact commands in the LOG):
  `cargo fmt --all --check`, `cix fmt --check examples`, warning-denied
  clippy, full workspace tests, tour regen + drift,
  `devenv shell -- nix run .#progressive-vm-check` for what the diff
  selects. Prove the cleanup: a test or receipt that a failed build
  leaves no scratch behind.
- Expect track/stopdispo in flight on main — resolve merges
  semantically yourself.
- Commit granularly; leave the branch clean. Do not merge to main.

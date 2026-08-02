# track/devfix — CI red after CIP-78: PrivateDevices breaks UserFull on GH runners

Read AGENTS.md first. Fix round for the devices merge (`dc27687`).
Work in `.worktrees/devfix` on branch `track/devfix`. Keep
`crates/cix-run/LOG.md` current.

**Symptom**: CI `test` job red on main since the devices merge:
`tour_matches_committed_document` — in CI's generated chapter 3 the
`cix ps` after `cix run tour/tour-app:v1 --detach --user` is EMPTY
where the committed doc shows the active unit row. Local gates
(NixOS) are green everywhere.

**Hypothesis to VERIFY first**: `add_device_policy` now pushes
`PrivateDevices=yes` on every claim-less unit including UserFull
(crates/cix-run/src/unit.rs:339). On the GH runner (Ubuntu 24.04,
AppArmor restriction on unprivileged user namespaces) the user
manager cannot set up the /dev mount namespace; the detached unit
fails asynchronously (likely 226/NAMESPACE), the transient unit is
collected, `cix ps` shows nothing. It is the ONLY behavioral change
to claim-less units in that merge. Verify the mechanism, don't assume:
e.g. reproduce with an AppArmor-style userns restriction
(`sysctl kernel.apparmor_restrict_unprivileged_userns=1` in a scratch
VM or container), or start a non-collected scratch user unit with
PrivateDevices=yes and capture its Result. Record the evidence in the
LOG.

**Fix direction** (consistent with existing design, D13/D36):
- Extend the existing `HostCapabilities` probe machinery
  (crates/cix-run — the `private_pids_with_persistent_directories`
  pattern) with a user-manager PrivateDevices support probe.
- When unsupported: drop `PrivateDevices=` from UserFull units through
  the existing `UnitDegradation` reporting path (warning printed, tour
  normalization swallows host-specific degradation detail — see
  `normalize_swallows_every_host_specific_degraded_fallback_detail`).
- System-mode behavior unchanged; hosts where UserFull PrivateDevices
  works keep it. Do not blanket-remove the property.
- The committed tour must be generated so it matches on BOTH host
  classes (the ps row must show the running unit either way).

Tests: unit test for the new probe + degradation path; ensure the
sync (non-detach) capability_failure fallback also covers this
failure class or document why it cannot trigger there.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Note honestly in the LOG that
the failing environment is CI-only: the orchestrator merges and
watches CI as the final verdict. Commit on this branch when green.

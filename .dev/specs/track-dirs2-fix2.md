# track/dirs2 fix round 2 — host-backed writes never reach the host path

Status after fix round 1: the orchestrator replaced the fixed-sleep
asserts with `wait_until_succeeds(..., timeout=60)` (committed on this
branch) and re-ran the FOCUSED scenario on an otherwise idle machine:
`test -f /tmp/dirs2/host-state/host-state` times out after 60s. This is
NOT a race: the marker never appears. The unit is active, never
restarts, and happily writes... somewhere else.

Diagnose for real, in-VM evidence first:

1. Where do the fixture's writes actually land? Run the scenario
   interactively (or add temporary diagnostics): `find / -name
   host-state` on the VM, the host service's mount table
   (`cat /proc/<pid>/mountinfo`), and `systemctl show` of the emitted
   Bind/TemporaryFileSystem/StateDirectory properties. Prime suspects:
   - the leg-1 overlay/full-host-mirror machinery stacking OVER the
     compose-emitted `BindPaths=`, so writes land in the private
     mirror;
   - the bind destination mountpoint being uncreatable in the ro
     namespace (the exact class of the leg-1 226/NAMESPACE catch) and
     the failure being swallowed;
   - the manifest `data` declaration for `/media` not reaching the
     compiler the way `state` does.
2. Fix the PRODUCT (generation/compile seam), not the scenario: a
   host-backed dir must make the operator path the real, writable view
   at the declared path, stacked correctly relative to the private
   machinery. Add a unit-level test pinning the property ORDER if
   ordering is the mechanism.
3. Explain the receipt divergence honestly: your two focused receipts
   reported this scenario green while byte-identical content times out
   here. If the scenario outcome is host-state-dependent or
   nondeterministic, that is itself a defect — find the mechanism (do
   not shrug it off this time; "unrecoverable" is not available while
   you can re-run both variants).

Gate: focused `nix build .#checks.x86_64-linux.scenario-dirs2
--no-link -L` until green, then the FULL
`devenv shell -- nix flake check -L`. Exact repros in
crates/cix-compose/LOG.md. Commit on this branch when green.

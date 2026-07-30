# track/composefallback — compose gets cix run's loud degraded path (D36 class, systemd 261)

Read AGENTS.md first. Authoritative: D36 + the D13 loud-degradation doctrine. Evidence:
`git show track/scenarios:nix/LOG.md` (terra's diagnosis, 2026-07-30 entries) — on the
NixOS-test VM's systemd 261, a compose-generated system unit combining
`DynamicUser=yes` + `PrivatePIDs=yes` + `StateDirectory=` + unix-edge writable
`BindPaths=` fails before exec with `226/NAMESPACE` ("Failed to allocate user
namespace"). The same stack works on host systemd 257; `vm-dogfood` (no such combo) is
green on 261. `cix run` has a loud fallback tier for unrealizable hardening; compose
generation has none — that gap is this track.

## Work

1. **Minimal repro + upstream check.** Isolate the minimal failing property set on
   systemd 261 (start from terra's evidence; bisect properties in a tiny VM check).
   Search systemd upstream issues/NEWS for 258–261 changes to PrivatePIDs/DynamicUser/
   namespace setup; record findings in your LOG (if it smells like an upstream bug,
   draft the issue text in the LOG — filing is Mathijs's call).
2. **Library-level loud degradation.** In the shared unit generator (crates/cix-run
   library surface that compose consumes), add host-capability-aware fallback for this
   class: when the host cannot realize the combination, drop the MINIMAL offending
   property (expected: `PrivatePIDs=`), exactly once, and loudly —
   (a) a warning on `cix up` naming unit + dropped property + reason,
   (b) the degradation recorded in the generation manifest so `cix compose`
   inspection shows it (same honesty pattern as D13/D36 in run).
   Never degrade silently; never degrade on hosts that support the combo. Detection:
   prefer an explicit capability probe (systemd version + a cheap realization test)
   over parsing activation failures; follow the probe pattern the run crate already
   uses for D36. Keep the probe overridable for tests (env or injected), so both paths
   have synthetic coverage on any host.
3. **Regression check in the VM.** Add ONE small flake check (`compose-fallback-vm` or
   fold into an existing check file you own — do NOT touch track/scenarios' files): a
   minimal compose stack with state dir + unix edge activates green on the VM's
   systemd 261, with the loud degradation observable in `cix up` output and manifest.
   On capable hosts the same code path degrades nothing (synthetic test).

## Scope & fencing

crates/cix-run (generator + probe) and crates/cix-compose (warning surface) only.
track/scenarios (terra) owns nix/scenarios/*; your VM check lives in its own file.
No changes to cix-index or cix-cixfile.

## Gate (exact repro commands in crates/cix-run/LOG.md)

`cargo fmt --all --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`cargo test --workspace` · tour regenerated (expect possible diffs only if warnings
surface in toured flows — review honestly; normalize host-varying detail per the
established tour lessons) · `nix build .#checks.x86_64-linux.vm-dogfood --no-link` ·
your new VM check green.

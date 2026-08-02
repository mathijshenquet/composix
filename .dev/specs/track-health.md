# track/health — CIP-79: READINESS/LIVENESS on native systemd

(Supersedes the pre-CIP design-position spec that lived here; CIP-79 is
the D-number it was waiting for, and it changed the mechanics: watchdog
instead of timer units, LIVENESS as restart opt-in.)

Read AGENTS.md first. Authoritative: docs/cips/0079-health.md (§3 + §5
Decision; it amends D48(c)). This is the biggest open implementation —
you have latitude on internal structure; the external contract below is
fixed. Work in `.worktrees/health` on branch `track/health`. Keep
`crates/cix-run/LOG.md` current. NOTE: track/devfix runs concurrently
in crates/cix-run (capability probe + PrivateDevices degradation) and
track/ergo in crates/cix-cixfile (FROM attrs, lock env) — expect a
main-merge before your gate; keep your parser additions and unit-gen
changes cleanly separated from those seams.

1. **Directives** (Cixfile, on SERVICE/APP):
   `READINESS http :8080/healthz IN 90s` | `READINESS tcp :5432 IN 60s`
   | `READINESS notify IN 90s`; `LIVENESS http :8080/livez EVERY 10s` |
   `LIVENESS tcp ... EVERY ...` | `LIVENESS notify EVERY 10s`. Probe
   types http/tcp/notify ONLY (no exec — YAGNI'd in the CIP). Manifest
   fields replace the v0 `health {exec, interval}` shape (D72: schema
   moves freely; migration-grade refusal of the old field). fmt
   support; parser diagnostics in house style; the CIP-mandated
   `LIVELINESS → did you mean LIVENESS` suggestion fixture.
2. **Compilation**:
   - `READINESS notify` → `Type=notify`; probe forms → unit stays
     `Type=exec` + `ExecStartPost=<cix probe await ...>` blocking until
     first success; `IN` → `TimeoutStartSec=`.
   - `LIVENESS notify` → `WatchdogSec=` (window = 3× EVERY, fixed);
     probe forms → a cgroup-resident `cix probe` pinger translating
     probe success into `WATCHDOG=1` (`NotifyAccess=all`). Declaring
     LIVENESS is the restart opt-in: emit `Restart=`/StartLimit
     properties on exactly the liveness-declaring units.
   - `cix probe` subcommand: native http/tcp prober (no curl, no
     shell), await mode and resident-pinger mode. Keep its closure
     footprint zero beyond the cix binary itself.
3. **Ban enforced**: compose schema rejects any health-condition
   vocabulary on edges (there is none to accept — assert the refusal
   with a schema test); ordering-follows-readiness comes free via
   structural edges (`After=` waits on start-job completion) — prove it
   in the VM scenario.
4. **`cix up` rollout semantics**: activation waits on start-job
   completion, so a failing READINESS surfaces as a failed `cix up` —
   assert this (rollout-status for free, per CIP).
5. **Docs**: docs/docker.md HEALTHCHECK + `condition: service_healthy`
   rows (❌ graph, ✅ probes honestly); docs/cixfile.md directive docs;
   docs/corpus.md rows blocked on CIP-79 re-graded per the ledger
   convention (desk unless you produce a receipt); tour touch only if a
   shown transcript changes.
6. **Tests**: parser + fmt round-trips; unit-gen snapshot fixtures
   (notify/http/tcp × readiness/liveness × system/user modes); prober
   unit tests; new `nix/scenarios/health.nix`: a member with http
   READINESS + LIVENESS comes up, `cix up` blocks-then-succeeds, a
   deliberately failing readiness makes `cix up` fail loudly, a hung
   member misses its watchdog and systemd restarts it (journal shows
   the CIP-83 exit-cause mapping "liveness watchdog missed").

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.

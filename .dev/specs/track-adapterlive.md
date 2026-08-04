# track/adapterlive — CIP-79 adapter liveness retention on systemd 257

Read AGENTS.md first (gate convention; synchronous receipts), then
CIP-79 (cips/accepted/0079-health.md) for the adopted semantics. Work in
the herdr worktree on branch `track/adapterlive`. Keep
`crates/cix-run/LOG.md` current under a dated `track/adapterlive` heading.

The recorded open item (docs/open-questions.md "Open for agents"): on
systemd 257 the cix-owned HTTP/TCP liveness adapter's `ExecStartPost`
parent exits successfully, but its forked resident pinger is not retained,
so a healthy service later hits `WatchdogSec` and gets killed as unhealthy.
Evidence trail: the Mastodon receipt's synchronous-receipt section
(corpus/migrate/mastodon/receipt.md#synchronous-receipt).

## Deliverables

1. **Reproduce first.** Determine the systemd version in the flake's VM
   universe and in CI. If the in-tree VM cannot run systemd 257, say so
   early in the LOG and evaluate the fix against the documented 257
   behavior (cgroup cleanup of ExecStartPost descendants) — do not build a
   bespoke out-of-tree reproduction rig without reporting the cost first.
2. **Mechanism decision, then fix.** Constraints: LIVENESS/READINESS
   manifest semantics are CIP-79-adopted and unchanged; no new manifest
   surface. Candidate mechanisms to weigh in the LOG before implementing
   (pick the least machinery that survives systemd's descendant cleanup):
   a supervised companion unit bound to the service (PartOf/BindsTo) that
   runs the pinger as a first-class process, versus keeping the fork but
   re-parenting it somewhere systemd is documented to retain, versus a
   version gate that disables the resident pinger on >=257 with an honest
   ledger note (last resort — it silently weakens liveness). If the clean
   fix requires a design amendment, stop and report.
3. **Regression protection**: the focused health VM scenario must assert
   that a healthy service under LIVENESS survives well past the watchdog
   window, on the systemd version the VM actually has; note explicitly
   which versions the assertion covers.
4. Ledger currency: update the docs/open-questions.md entry and, if
   behavior changes, the affected docs/docker.md health rows in the same
   track. Grep `corpus/migrate/*/GAPS.md` for liveness-related gaps and
   flip any exhibiting case stale per the AGENTS.md extension.

FENCE: your domain is health/liveness adapter code in crates/cix-run (and
its unit generation), the health VM scenarios, and the named ledger
entries. Do not touch corpus Cixfiles, docs/corpus*, docs/migrate.md,
cips/, or netns/pod wiring (track/netnsrace runs concurrently there). If
the fix genuinely requires crossing into shared unit-generation code that
netnsrace also touches, note it in the LOG and proceed carefully — the
orchestrator resolves overlap at merge.

## Gate

Standard agent tier (fmt, examples fmt, warning-denied clippy, full
workspace tests, tour regen+drift) plus the FOCUSED health VM scenarios.
Receipts are synchronous exit statuses with exact repro commands.

# track/stopdispo — STOPSIGNAL/stop-timeout implementation + docker.md disposition application

Read first: `cips/dispositions.md` (the blessed 2026-08-04 batch — this
track applies it), `docs/docker.md`, `AGENTS.md` (gates, receipts).

## Scope

1. **STOPSIGNAL** (the one disposition with an implementation): Docker's
   `STOPSIGNAL <signal>` directive is accepted in Cixfiles and maps to
   systemd `KillSignal=` in the generated unit; the compose-side stop
   grace period (docker compose `stop_grace_period`) maps to
   `TimeoutStopSec=`. This is the blessed mechanism — a 1:1 systemd
   mapping, no new semantics. Grammar: `STOPSIGNAL SIGQUIT` (Dockerfile
   spelling; validate against known signal names). If any surface
   question arises that the disposition text does not answer (e.g.
   where the compose timeout field lives in compose.json), propose the
   minimal conventional answer in your report and flag it — do not
   silently invent grammar beyond these two mappings.
   Consumers: adminer and nginx GAPS.md note the upstream signal
   contract — after landing, flip those bullets/status per the ledger
   rule (`Status: stale — regenerate with STOPSIGNAL` where the case
   would change).
2. **docker.md re-marking** — apply every blessed verdict from
   `cips/dispositions.md` to its docs/docker.md row: docker cp ❌,
   --name ⏳, STOPSIGNAL ✅ (after step 1 lands; stop timeout per what
   you implement), namespace-sharing → pods (standalone flags ❌),
   restart-tuning later, docker init ⏳, Docker Offload ❌,
   AppArmor/SELinux out-of-scope, Desktop ECI ❌, authorization
   plugins ❌ (reconciler era), Engine API reconciler-era, remote
   contexts via ssh (sugar ⏳), docker mcp ❌, capabilities
   claim-by-claim. Each row cites the disposition record
   (`cips/dispositions.md`) so the ledger and the record stay linked.
   Keep row prose in docker.md's existing honest voice; verdicts are
   decided — do not re-litigate them.

Out of scope: ENV grammar (parallel track/envgrammar), EXPECT sweep
(parallel track/cip102). Expect both to be in flight — resolve any
merge from main semantically yourself.

## Discipline

- Branch `track/stopdispo`, this worktree. Log: `crates/cix-run/LOG.md`,
  timestamped, append-only.
- Gates (synchronous exit-status receipts, exact commands in the LOG):
  `cargo fmt --all --check`, `cix fmt --check examples`, warning-denied
  clippy, full workspace tests, tour regen + drift check,
  `devenv shell -- nix run .#progressive-vm-check` (STOPSIGNAL should
  get a scenario assertion: generated unit contains the KillSignal=
  you declared).
- A receipt is a synchronous exit status you observed.
- Commit granularly; leave the branch clean. Do not merge to main.

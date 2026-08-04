# Open questions

Rewritten 2026-08-04 (Mathijs: resolved items out, every entry
self-contained, larger items promoted to CIP drafts). Rules of this
document: an entry either carries enough context to act on without
opening another file, or it does not belong here. Resolved work lives
in `.dev/LOG.md` and the CIP changelogs, not in this file. Design-sized
questions live in `cips/draft/` — this file only points at them.

## Open drafts in the inbox (each self-contained)

- [env-equals](draft/env-equals.md) — switch ENV to the `NAME=value`
  grammar; prior work (bash/dotenv/docker/systemd/make) is unanimous,
  our spaced form is the outlier.
- [build-args](draft/build-args.md) — lock-pinned ARG: CLI overrides
  are recorded in the lock, so file+lock stays the whole truth while
  Cixfiles become parameterizable.
- [volatile-fetch v3](draft/volatile-fetch.md) — EXPECT is for stable
  artifacts only; volatile fetches use TOFU consumed pins + upstream
  checksums in RUN; diagnostic teaches this on mismatch.
- [tmp-relocate v2](draft/tmp-relocate.md) — the requested design
  round: cleanup-on-every-exit is the primary fix (nix-style), big
  trees go to /var/tmp per systemd's file-hierarchy guidance.
- [fhs-interpreter → deferred/fixup-elf](deferred/fixup-elf.md),
  [fetch-checksum-crosscheck](deferred/fetch-checksum-crosscheck.md),
  [compose-syntax](deferred/compose-syntax.md) — parked by decision.

Adopted out of this file today: CIP-96 optional-env (bare `ENV NAME`),
CIP-97 granular-degradation (batched systemd-analyze verify probe),
CIP-98 artifact-root-collision (role dirs anywhere, docker-volume
nesting), CIP-99 lock-scale (subtree aggregation, 4x-checked).
Implementation tracks for 96–99 are queued.

## Ledger dispositions — blessed 2026-08-04 (application queued)

Mathijs blessed the batch with two exceptions, which became their own
drafts (env-equals, build-args above). The blessed verdicts — docker cp
❌, --name ⏳, STOPSIGNAL/stop-timeouts ⏳ (small mechanical track:
KillSignal=/TimeoutStopSec=), namespace-sharing via pods only,
restart-tuning later, docker init ⏳, Docker Offload ❌,
AppArmor/SELinux out-of-scope, Desktop ECI ❌, authorization plugins
never (reconciler era), Engine API reconciler-era, remote contexts via
ssh, docker mcp ❌, capabilities claim-by-claim — now need their
docs/docker.md rows re-marked accordingly: queued as one small
mechanical track (also picks up the STOPSIGNAL implementation, which
has two corpus consumers).

## Era-parked (deliberate deferrals — context, then silence)

- **Publish era** — everything about *sharing* indexes publicly: push
  to a remote index, auth/login, signing entries, mirrors and
  pull-through caches, a hub with search/webhooks, SBOM/attestation
  exchange. Parked as one coherent future era (recorded as decisions
  D17/D35 in docs/design.md); today's serve/pull covers the local and
  trusted-network story.
- **Named-network era** — first-class network objects: service DNS
  names, `talks-to` allow-lists between services, cross-composite and
  multi-host networking, per-service IP/DNS/hostname options. Parked
  as decisions D26/D27. What IS built (CIP-86): per-subtree shared
  network namespaces (pods), port publish, egress with persistent
  addressing, and closed-root resolver projection.
- **Compose v1+** — replicas/scale, resource limits, reusable config
  objects, live update of a running composite. Parked in the compose
  scope decision (D30): v0 is deliberately lean and each omission is
  recorded in the docker.md ledger.

# Open questions

Rewritten 2026-08-04 (Mathijs: resolved items out, every entry
self-contained, larger items promoted to CIP drafts). Rules of this
document: an entry either carries enough context to act on without
opening another file, or it does not belong here. Resolved work lives
in `.dev/LOG.md` and the CIP changelogs, not in this file; blessed
ledger verdicts live in [dispositions.md](dispositions.md). Design-sized
questions live in `cips/draft/` — this file only points at them.

## Open drafts in the inbox (each self-contained)

- [nodes-and-edges](draft/nodes-and-edges.md) — the language epoch:
  argv-first steps (one command per node), heredoc-only shell,
  LET/ARG + per-node WITH edges (builder-ENV banned, leaf-ENV stays),
  adjacency clauses; session-shells rejected with argument.
- [phase-blocks](draft/phase-blocks.md) — explicit `{ }` for
  BUILDER/ITEM/SERVICE/APP (Caddyfile/HCL lineage); key call is epoch
  coupling with nodes-and-edges.
- [build-args](draft/build-args.md) — closed-matrix ARG with partial
  per-cell lock; open: args×tagging, enumeration syntax.
- [fmt-key-neutrality](draft/fmt-key-neutrality.md) — `cix fmt`
  changed a builder chain key (haproxy repro preserved unformatted);
  keying-fundamentals fix, prerequisite to any epoch sweep.
- [pnpm-wall](draft/pnpm-wall.md) — the npm/pnpm ecosystem-fetch wall,
  five exhibits (homer, dozzle, verdaccio-cold, directus, filestash).
- [k8s-wave](draft/k8s-wave.md) — wave-1 case selection for the
  drafted docs/migrate-k8s.md teaching contract.
- [fhs-interpreter → deferred/fixup-elf](deferred/fixup-elf.md),
  [fetch-checksum-crosscheck](deferred/fetch-checksum-crosscheck.md),
  [compose-syntax](deferred/compose-syntax.md) — parked by decision.

Adopted 2026-08-05 and ALL IMPLEMENTED same day: CIP-100..102 (env
grammar, tmp-relocate incl. liveness-guarded sweeping, volatile-fetch)
and the audit batch CIP-103..108 (103 complete through the Workspace
and MemoEngine legs; context/FETCH legs and 104 queued) plus CIP-109
probe-url. Ribbon borderline trio (NATS/renovate/whoami ✅) awaits
Mathijs's confirm-or-flip.

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

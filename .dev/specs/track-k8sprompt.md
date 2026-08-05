# track/k8sprompt — design the k8s teaching prompt (draft output only)

The k8s corpus axis has a skeleton and CANDIDATES
(`corpus/migrate/k8s/`) but no teaching prompt — that is the blocker
for k8s wave 1. Read: `docs/migrate.md` (the docker teaching contract
and its evolution discipline), `corpus/migrate/k8s/{README,
CANDIDATES}.md`, docs/design.md compose decisions (D30/CIP-85/86 —
pods, netns, health, listeners), and 2-3 representative k8s
candidates' manifests.

Deliverables (drafts only — NO implementation, NO wave launch):
1. `docs/migrate-k8s.md` DRAFT: the teaching contract for translating
   k8s manifests (Deployment/Service/ConfigMap/probes/volumes) into
   compose.json + per-service Cixfiles. Reuse docker-migrate.md's
   voice and receipt discipline; map k8s concepts to existing cix
   surfaces (probes -> READINESS/LIVENESS per CIP-109 URL form,
   volumes -> role dirs/host:, Services -> listeners/ports,
   replicas -> honestly out of scope per D30). Every k8s concept the
   language cannot express gets an honest "not expressible — record
   as gap" instruction, not an invention.
2. A short `cips/draft/k8s-wave.md` (CIP-light) proposing wave-1 case
   selection (3-4 candidates with reasons) and any language gaps the
   prompt design surfaced, for Mathijs's adoption.

Discipline: branch `track/k8sprompt`, LOG `corpus/migrate/LOG.md`.
Read-only toward src/; gates: `git diff --check` only (docs/drafts
diff). Clean branch; do not merge.

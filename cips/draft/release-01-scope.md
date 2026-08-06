# release-01-scope — what 0.1 is, and what it is not

Status: **draft** (2026-08-06; requested by Mathijs — "epoch as
pre-release must" from the 0.1 direction discussion, plus his
documentation call recorded below).

## 1. The problem

"0.1" currently means everything and nothing: the alpha grows by
CIP, but no recorded line says what must be true before composix is
shown to anyone outside this repo. Without a scope, the epoch's
migration churn, the docs, and the naming question have no shared
deadline semantics — and the honest-ledger culture needs a boundary
to be honest *about*.

## 2. Prior work

- **Semver 0.x custom**: 0.1 is the first "someone else may look"
  cut — API instability expected, honesty about it required.
- **Our own D72**: alpha manifest = version 0; all manifests and
  Cixfiles live in this repo ("voor nu is het ons feestje"). 0.1 is
  the moment that stops being fully true.
- **The corpus doctrine** (docs/corpus.md): human-consumable first —
  "today the dev-loop instrument, later adopter-facing
  documentation". 0.1 is exactly that "later"'s first instance.

## 3. Recommendation

0.1 = the first coherent outside-readable cut. IN, each already
adopted or in flight:

1. **The language epoch landed** (CIP-110..113: fmt-key-neutrality,
   nodes-and-edges, phase-blocks, build-args) — one corpus sweep, one
   migrate.md rewrite, all 30+ cases green-or-honestly-walled under
   the new grammar. The epoch is the pre-release MUST: nobody should
   learn the old ENV/bash-RUN language from a 0.1.
2. **The pnpm wall resolved** (frozenStore route receipts + the
   problem-class diagnostics; pnpm-wall draft adopted as CIP once
   receipts land). The largest real-Dockerfile population must not
   hit a mute wall in the first hour.
3. **Naming settled** (companion draft: naming-table; Mathijs leans
   "everything composix") — renames are epoch-cheap now and
   embarrassing after 0.1.
4. **Documentation: full redo, personally by Mathijs** (recorded
   2026-08-06: "ik wil heel graag alle documentatie zelf redoen en
   reviewen als 0.1 item"). Every adopter-facing page — tour,
   migrate.md, README, docs/docker.md framing — gets a Mathijs
   rewrite/review pass; agent-written prose may remain only where he
   explicitly blesses it. Orchestrator prepares the inventory and
   the supporting fact-checks; the voice is his.
5. **Honest-gap surfacing**: docs/docker.md and the corpus ledger
   presented as the feature ("we tell you exactly what does not
   work"), not hidden.

OUT (explicitly post-0.1, unchanged from the standing backlog): the
publish era, the reconciler daemon, D26/D27 named networks and
`talks-to`, the phase-2 closed-root flip, composite netns (D23), TAG
(deferred), LET-lists, k8s wave ≥2.

## 4. Open questions

- **k8s wave 1**: inside 0.1 (the bridge story doubles the audience)
  or the first post-0.1 item (protects the epoch's focus)?
  Orchestrator's lean: post-0.1.
- **Version semantics**: does 0.1 bump `cixManifest` past D72's 0, or
  is 0.1 a repo/tag event with manifest 0 intact until compat
  actually breaks?
- **Release artifact**: what IS 0.1 concretely — a git tag + README
  claim, a nix flake output others can `nix run`, crates.io, a binary
  release? (Lean: tag + flake; nothing that creates a support
  treadmill.)
- **Gate**: does 0.1 require the full closed-root audit posture on
  every corpus case, or is the current mixed closed-root ledger
  honest enough to ship as-is?

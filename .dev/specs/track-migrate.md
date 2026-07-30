# track/migrate — distill a Dockerfile→Cixfile prompt against an executable loss

Designed by Mathijs 2026-07-30. The ML framing is literal: the **prompt is the model**,
the **wild Dockerfiles are the data**, the **dual health check is the loss**, and
**prompt length is the regularizer**. This track is also the empirical adversarial
re-analysis of docs/corpus.md's desk grades — receipts supersede ribbons.

SEQUENCING: conversion rounds start only after track/blocks (D47) merges — every
produced Cixfile must be written in blocks-and-binders syntax or it rots on arrival.
The sourcing sweep and harness scaffolding may start immediately.

## Artifacts

- **`docs/migrate.md` — THE PROMPT.** A README that teaches Dockerfile→Cixfile
  conversion to a fresh reader. As minimal as possible while achieving good loss.
  This file doubles as future user documentation; its git history is the distillation
  log. HARD RULE: the prompt may state general lessons only — naming specific images
  or hardcoding per-image fixes is overfitting and forbidden.
- **`corpus/migrate/<name>/`** per pair — the pair root contains ONLY the pair
  artifacts (Dockerfile, SOURCE, Cixfile, Cixfile.lock, check.sh, receipt.md);
  any Docker build-context files (entrypoints, configs, or a whole fetched app
  tree) live under **`context/`**, never loose in the root (layout convention,
  Mathijs 2026-07-30 — a pair must be readable at a glance):
  - `Dockerfile` (verbatim from the wild) + `SOURCE` (url + fetch date + image ref)
  - `Cixfile` (the conversion, D47 syntax)
  - `check.sh` — the minimal dual health check: `./check.sh docker` builds+runs the
    original image and probes it; `./check.sh cix` builds+runs the cix item under the
    normal hardening and runs the SAME probe body (curl / redis PING / psql SELECT 1 /
    binary --version…). Bounded timeouts, clean teardown, exit 0/1. Minimal by design:
    the probe proves the service does its one central thing, not full equivalence.
  - `receipt.md` — transcripts of BOTH sides passing (commands verbatim, key output
    lines, image digest + item store path). A pair without a dual receipt is not a
    pair.
- **`corpus/migrate/LOG.md`** — append-only round journal: batch composition, per-pair
  verdict (build-fail | run-fail | check-fail | pass), round loss, failure taxonomy,
  prompt edits with their justification, prompt length per round.

## Sourcing (may start now; research task)

Build `corpus/migrate/CANDIDATES.md`: ~40 wild Dockerfiles ranked by (popularity ×
build speed). Prefer Docker Hub's most-pulled images and popular self-hosted apps;
PREFER FAST BUILDS (≤ ~2–3 min: prebuilt-binary unpacks, single-ecosystem app builds,
alpine/slim bases) — "we zijn geen build farm aan het maken." Exclude: CUDA/ML,
browser images, multi-hour compiles. Note per candidate: source URL, what it exercises
(the corpus.md mechanism vocabulary), expected difficulty. Spread easy→nasty, but
weight toward the middle: trivial teaches the prompt nothing, nasty teaches it
everything at once.

## The loop (per round; batch sizes N = 1, 2, 4, 8, 16)

1. Take the next N candidates (plus at most one retry of a prior failure after a
   prompt refinement).
2. **A fresh converter agent per batch** — this is the validity condition. The
   converter receives ONLY: `docs/migrate.md`, the Dockerfile(s), a built `cix`
   binary, and the check-harness contract. It does NOT get the composix source, design
   docs, or tour. Otherwise the round measures the agent's context, not the prompt.
   Converter model: terra (consider a luna round late in the track as a
   prompt-robustness test — a weaker reader is a stricter grader of the README).
3. Converter produces Cixfile + check.sh + runs both sides + writes receipt.
4. **Spot-check (verifier ≠ converter)**: triage EVERY failure into prompt-gap
   (README unclear/missing lesson) vs corpus-gap (image needs a mechanism cix lacks —
   feeds design.md, cite the docs/corpus.md row) vs product-bug (file a track);
   AND adversarially audit a sample of PASSES for vacuous probes (a check that would
   pass on a broken service is worse than a failure — kill and redo such receipts).
5. Refine the prompt: add the smallest general lesson that fixes the round's
   prompt-gaps; run deletion experiments when loss allows (can the prompt shrink
   without loss rising?). Record (loss, prompt length) per round in the LOG.
6. Stop when a round at N=16 holds loss low with a stable prompt, or when remaining
   failures are all corpus-gaps (those are design input, not prompt input).

## Output

A strong minimal `docs/migrate.md`, ≥ ~20 dual-receipted pairs in `corpus/migrate/`,
a failure taxonomy that grades docs/corpus.md's desk ribbons against reality, and a
list of product gaps discovered.

## Explicit follow-up (OUT of this track)

`cix migrate` (sol): the tool that automates the prompt; `corpus/migrate/` becomes its
test suite on day one. Prerequisite design round: **equivalence modulo syntactic
difference** (when is generated-Cixfile ≡ corpus-Cixfile? — candidate: both build, and
their items pass the pair's check.sh + closure-shape comparison; exact definition is a
D-number to take with Mathijs).

## Fencing

New top-level `corpus/` + `docs/migrate.md` only; no crate changes. Docker daemon
confirmed available on this host (29.5.2) for docker-side receipts. Keep docker-side
runs unprivileged-by-default; a candidate needing privileged/devices is recorded and
skipped (that's a corpus-gap datum, not a challenge to overcome).

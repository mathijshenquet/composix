# lock-scale — Cixfile.lock growth on big-ecosystem builds (CIP-light)

Status: **draft, CIP-light** (2026-08-04; promoted out of open-questions).

**Problem.** The lock records per-step read/output observations
(CIP-87) plus dev-env snapshots (CIP-88). On node-ecosystem cases that
is enormous: directus grew the lock by ~148k lines, parse-server's is
~400k lines, watchtower ~107k. Diffs, review, and the corpus browser
all pay for it; the observations are correct, just uncompressed.

**Proposal shape (to design).** Aggregate where identity permits: e.g.
one digest per directory subtree instead of per-file rows where the
consumer reads the subtree as a unit, or a compact section format for
step observations. Constraint: pins/EXPECT stay reviewable per
dependency (that diffability is CIP-94's core pitch); only the bulk
observation records compress.

**Effort.** Medium — format design + migration of readers; alpha, so
no compat needed once chosen.

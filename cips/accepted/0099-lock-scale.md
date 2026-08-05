# lock-scale — Cixfile.lock growth on big-ecosystem builds (CIP-light, v2)

Status: **CIP-99, adopted 2026-08-04** (CIP-light; "lijkt me correct"
contingent on the 4x coherence check below — which came out clean).

**What the bytes actually are** (parse-server, 400,355 lines, measured):

```
Cixfile.lock
├─ inputs         8 lines   (universe pin)
├─ fetches       14 lines   (FETCH pins — never the problem)
├─ memo          54 lines   (item outputs)
├─ devEnvs        8 lines
├─ outputs        6 lines
└─ stepMemo 400,262 lines   (100% of the bulk)
   └─ builder:deps:6  = 337,840 lines   (the npm-install step)
      ├─ reads   266,750 lines = 23,719 per-file records
      │    23,717 of them under node_modules/…
      │    {"kind":"directory","hash":"…"} / file hashes, one per path
      └─ changes  71,087 lines (same shape, written paths)
```

So: NOT download pins — these are CIP-87's traced read/change
observations of offline steps, one JSON record per file, and ONE step
over node_modules is 84% of the file.

**Proposal.** Subtree aggregation: when a step's reads/changes cover a
subtree (dir-hash already computed per directory record), collapse the
per-file records under it into that one subtree digest; expand only
where later steps read narrower paths. Per-dependency FETCH pins and
their diffability are untouched (they are 14 lines). Expected effect on
parse-server: ~400k → hundreds of lines.

**Effort.** Medium: memo read/validate paths learn subtree records;
alpha, no compat.

## Decision — with the requested 4x coherence check

Adopted 2026-08-04. The four turns, honestly:

1. *Validation cost*: memo re-validation walks the same files either
   way (a subtree digest is computed over its children); records
   shrink, work does not change. Coherent.
2. *Invalidation granularity*: IDENTICAL — the step consumed the whole
   subtree, so any changed byte invalidates it under both
   representations. The lock does NOT become coarser where it matters.
   What coarsens is change ATTRIBUTION in reports: naming which file
   changed needs a live re-walk diff at report time (compute traded
   for lock size). Acceptable.
3. *Cross-step independence*: aggregation is per-step over that step's
   own read set; a later step reading one file records one file.
   Consumed-output records (what items COPY) stay per-path. No
   coupling. Coherent.
4. *The correctness edge*: collapse ONLY directories whose recorded
   reads literally cover the entire subtree, bottom-up; a partially
   read directory keeps per-file records for the read entries. The
   digest is defined over sorted (relpath, kind, hash) of children —
   semantically lossless compression, never a heuristic. One recorded
   caveat: D69 volatile-fact reporting keeps naming individual files
   (those sets are small); aggregation applies to stable-validated
   subtrees only.

Effect target: parse-server 400k -> hundreds of lines; per-dependency
FETCH pins and their diffability untouched.

## Changelog

- 2026-08-05 — Implemented: stable fully observed read trees serialize as one
  recursive `subtree` digest, while complete output trees serialize as one
  replay root. The digest is over sorted `(name, kind, hash)` children;
  partial and volatile observations remain per-path. Clean-HEAD controls
  retained identical output store paths while parse-server and phpMyAdmin
  locks shrank substantially; echo-server’s partial tree stayed precise.
- 2026-08-05 — Corrected the workspace-root aggregation boundary: `.` now
  covers its recorded descendants, while negative observations remain explicit
  so creating a previously absent path still invalidates the memo. This fixes
  a lossless-compression criteria gap exposed by the it-tools lock.

# volatile-fetch — make EXPECT consumed-set-aware (v2)

Status: **draft v2** (2026-08-04; v2 after Mathijs: "waarom hier niet
ook read set intersecten?" — he is right, and it dissolves the original
proposal's lint/teaching surfaces).

## 1. The problem

Whole-tree EXPECT hashes every fetched byte, including bytes nothing
consumes: GitHub release JSON download counters (traefik), package-
manager cache subsets (dozzle sumdb tiles, node_modules variance), a
mirror-varying tarball (phpmyadmin). Such pins fail on any true
refetch. Meanwhile the TOFU path (no EXPECT) ALREADY pins only
consumed paths — the volatility problem exists precisely because
EXPECT does not intersect with the consumed read set.

## 2. Prior work

CIP-87/D69 built the consumed-set machinery: per-path content records
in the lock, narrowing pins to what downstream reads. v1 of this draft
proposed teaching normalization idioms plus a volatile-shape lint —
respectively mediocre DX and a maintenance surface (Mathijs), both now
demoted.

## 3. Recommendation

`EXPECT` verifies the **consumed subset**: the declared hash covers the
per-path content records of exactly the paths later steps consume,
computed from the same lock records TOFU uses. Volatile side-bytes
become irrelevant to both trust modes.

The honest trade-off: the consumed set changes when consumers change —
adding a `COPY ${fetch}/other-file …` shifts the expected hash. The
diagnostic makes re-blessing precise: on mismatch, print the old and
new consumed path lists and their per-path hashes, so the author sees
"you now also consume X" versus "content of Y changed" at a glance —
these are distinguishable cases, and only the second is a trust event.

Corpus sweep after landing: traefik/phpmyadmin/dozzle-class cases
re-pin under the new semantics; the teaching paragraph in migrate.md
shrinks to "EXPECT covers what you consume".

## 4. Open questions

- Spelling of a whole-tree escape (if any consumer genuinely wants
  everything: `COPY ${fetch}/ .` already expresses it naturally — is
  an explicit flag ever needed?)
- Migration: existing whole-tree EXPECT values fail under consumed-set
  semantics; alpha rule says hard switch with a clear error naming the
  recompute command — confirm.

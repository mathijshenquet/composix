# CIP-102: volatile-fetch — EXPECT is for stable artifacts; teach and diagnose, do not re-engineer

Status: **accepted** (2026-08-05. v3 after Mathijs's sharper question:
"waarom zou je überhaupt expliciete EXPECT schrijven als het ding dat
je fetched niet stable is?" — exactly; v2's consumed-set EXPECT is
withdrawn as complexity papering over a usage error).

## 1. The problem, reframed

The corpus failures (traefik release-JSON counters, phpmyadmin mirror
variance, dozzle cache subsets) all wrote `EXPECT` over content that is
not stable. That is the error itself: EXPECT is the author's precise
whole-tree claim and belongs on genuinely stable artifacts (release
tarballs, tagged files). For everything else the TOFU path already
does the right thing — it pins only consumed paths, so volatile
side-bytes never enter the pin.

## 2. What was considered and dropped

- v1: normalization teaching + a volatile-shape lint — mediocre DX and
  a maintenance surface (Mathijs).
- v2: consumed-set EXPECT — dissolves the symptom but changes EXPECT
  semantics, adds re-blessing churn on consumer changes, and still
  encourages EXPECT where it does not belong.

## 3. Recommendation

1. **Teaching (one paragraph in migrate.md)**: EXPECT only what is
   stable; for volatile or ecosystem-managed fetches, use TOFU
   consumed pins and, when author trust is wanted, verify upstream's
   published checksum inside RUN (the corpus already does this:
   `sha256sum -c` against the vendor value — trust without pinning
   noise).
2. **The mismatch diagnostic teaches**: on EXPECT divergence, cix
   already names both hashes; add one line — if a refetch of unchanged
   upstream diverges, the fetched tree is volatile; drop EXPECT and
   rely on consumed pins (command shown), or pin a stable asset URL.
3. **Corpus sweep**: remove the wrongly-placed EXPECTs
   (traefik/phpmyadmin/dozzle class), keeping their RUN-level upstream
   checksum verifications; re-grade rows.

**Effort.** Small: one diagnostic line, one teaching paragraph, a
mechanical corpus sweep.

## Decision

Adopted as proposed in v3 (Mathijs, 2026-08-05: "prima"), with his
confirmation question answered by a corpus survey: the remaining
EXPECT consumers are indeed stable artifacts — caddy (pinned-commit
raw files + release tarball), adminer, memcached, nats, redis (release
assets). The exceptions are exactly the sweep list from §3 (traefik
release-JSON, phpmyadmin mirror pipeline), plus echo-server's
script-driven FETCH to audit during the sweep (it was a cold read-set
divergence case). "EXPECT only for stable" matches actual usage.

Changelog:
- 2026-08-05 — adopted as CIP-102.

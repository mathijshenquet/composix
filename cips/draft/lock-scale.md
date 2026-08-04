# lock-scale — Cixfile.lock growth on big-ecosystem builds (CIP-light, v2)

Status: **draft, CIP-light, v2** (2026-08-04; v2 shows the measured
shape per Mathijs's ask).

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

# artifact-import — one way to assemble an artifact's toolset

Status: **draft v2** (2026-08-04; v1 same day from Mathijs's corpus review,
v2 after his design round: STATEDIR-direct kills runtime-path links, LINK
itself is on the table, and the governing principle is *fewer ways to do
the same thing* — the current degrees of freedom are disorienting and
footgunny).

## 1. The problem

An artifact (SERVICE/APP/ITEM) that runs binaries from nixpkgs packages has
no canonical way to say so. The corpus exhibits **four** competing styles:

1. **LINK into `/bin` + bare START** (adminer, directus, echo-server,
   verdaccio, watchtower, whoami) — an *implicit self-import* that reads as
   magic and degenerates into LINK piles (wallos carries six).
2. **Interpolated START path, no assembly** (redis) —
   `START ${pkgs.redis}/bin/redis-server …`; clean for one binary, but the
   item a debugger enters is oddly empty.
3. **Whole-package COPY + hand-built PATH** (tomcat) — reimplements builder
   IMPORT at ad-hoc root paths.
4. **Wrapper scripts assuming style 1** (wallos setup/start).

Four spellings of one intent is the defect; the fix is to make one of them
the only blessed one.

## 2. Prior work

Builders already have the answer: `IMPORT` unions the packages'
`bin`/`etc`/`share` at `/bin`/`/etc`/`/share`, earlier wins, bare command
words are build-time checked. `docs/migrate.md` currently teaches style 1
for artifacts, which is where the piles come from.

On the "does interpolation vs linking matter technically?" question
(Mathijs, this round): no hard reason survives inspection. A symlink's
target *is* the store-path string, so link-assembly embeds exactly the same
references in the item as interpolated argv does; the closure walk sees
both; and content-addressing pain concentrates on *self*-references, which
neither form creates. The only principled argument left is uniformity —
which the governing principle says is sufficient.

## 3. Recommendation

**(a) IMPORT becomes universal.** Every block kind — BUILDER, SERVICE, APP,
ITEM — accepts `IMPORT ${pkgs.x} …` with identical union semantics. The
artifact's toolset is one visible line; bare `START`/`START_PRE` words stay
build-time checked against the assembled tree.

**(b) LINK dissolves into store-aware COPY.** COPY already knows when its
source is immutable store content; it may then link instead of
materializing as an implementation detail. `LINK x y` and `COPY x y` differ
today only in that choice, so one verb suffices:

```dockerfile
COPY ${pkgs.nginx}/conf/mime.types /etc/nginx/mime.types   # links under the hood
```

One concept is deleted from the language. Implementation must define the
union rule when a later COPY writes *under* a link-assembled directory
(materialize that subtree deterministically), and the migration spike
verifies nothing at runtime distinguishes symlink from materialized file
(realpath-sensitive apps get checked in the regeneration sweep).

**(c) One canon for executables.** With (a), the blessed form is
`IMPORT` + bare command words. Interpolated store paths in `START`/
`START_PRE` argv remain grammatically legal (FILE content and COPY sources
need interpolation regardless) but become a **lint**: the strict mode
warns and the teaching prompt never shows the form. Redis becomes

```dockerfile
SERVICE redis
  IMPORT ${pkgs.redis}
  STATEDIR /data
  START redis-server --dir /data --port 6379
```

Lint timing per Mathijs ("slightly YAGNI, left to you"): the lint lands
with the regeneration sweep, where mechanical generation makes it earn its
keep; no separate track before that.

**(d) State binds where the app expects it — v1's runtime-path LINK is
rejected.** `STATEDIR /directus/database` is the whole answer: role dirs
already accept any clean absolute path and already do the binding, so
redirect-symlink ceremony (builder `ln -s` dances *and* v1's proposed
`LINK /var/lib/… /app/…`) is slop against existing mechanism. Directus
declares three STATEDIRs at its native paths and deletes the ritual.

On landing: `docs/migrate.md` rewrites its runtime-toolset section around
IMPORT + bare names and the role-dir rule; the exhibiting cases (at
minimum adminer, directus, echo-server, filestash, memcached, nats,
redis, tomcat, verdaccio, wallos, watchtower, whoami) flip
`Status: stale — regenerate with artifact-import`.

## 4. Open questions

- **Top-level IMPORT spill** (Mathijs: "maybe"): may a prelude-level
  `IMPORT` apply to every block? Convenient for `${pkgs.bash}`-everywhere
  files, but it couples all blocks' keys to one line (any change rebuilds
  everything) and reintroduces an ambient-ish surface. Lean: block-local
  only at first; revisit if regenerated corpus files show painful
  repetition.
- **COPY union semantics** for writes under a link-assembled directory:
  materialize-on-write per subtree, or refuse and require narrower COPY
  sources? (Tomcat's tree is the test case.)
- Does `ENV PATH = …` replacing the default still make sense once IMPORT
  is the canon, or should explicit PATH become a lint alongside (c)?

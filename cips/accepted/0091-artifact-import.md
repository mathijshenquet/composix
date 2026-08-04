# artifact-import — one way to assemble an artifact's toolset

Status: **CIP-91, adopted 2026-08-04** (drafted, revised to v2 in Mathijs's
design round, and adopted the same day; governing principle: *fewer ways to
do the same thing*).

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

**(b) LINK dissolves into store-aware COPY** (Mathijs 50% sure — turned
over 4×, holds up if the rules below stay this clean; the implementation
track spikes it first and LINK survives as fallback if the spike sours).
`LINK x y` and `COPY x y` differ today only in symlink-vs-materialize at
assembly, so one verb suffices, **by rule rather than heuristic**: COPY
whose source is immutable store content links; COPY from build context
materializes. The mode is derivable from the Cixfile text alone.

```dockerfile
COPY ${pkgs.nginx}/conf/mime.types /etc/nginx/mime.types   # links, by rule
```

The four turns:

1. *Runtime mutation* — moot: under the closed root the item tree is
   read-only either way; writes live in role dirs regardless.
2. *Realpath visibility* — the residual risk. Linked binaries are the
   status quo (every LINK today is a symlink) and store-relative
   resolution usually helps, but a linked *application tree* changes
   upward/sibling resolution for realpath-walking runtimes (node's module
   walk is the canonical case). Mitigation: the regeneration sweep tests
   the whole corpus, and the escape is restructuring via a builder `cp` —
   no new syntax.
3. *Structural exception, statically known* — a role-dir/`DIR` mount
   point beneath a COPY destination requires real ancestor directories
   (nothing mounts under a symlink into the store), and a later COPY
   writing beneath a linked directory needs the same. Both conditions are
   visible at build time: materialize the destination chain in exactly
   those cases (or refuse with a suggestion — spike decides). This also
   answers directus: `COPY ${build}/dist /directus` + three STATEDIRs
   under `/directus` forces `/directus` to materialize, automatically.
4. *Distribution* — linking genuinely wins: referenced packages stay
   shared closure members across items (smaller NARs, dedup over the
   wire); materializing would inline the bytes per item.

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
  sources? (Tomcat's tree is the test case; §3(b) turn 3 leaves it to the
  spike.)
- Does `ENV PATH = …` replacing the default still make sense once IMPORT
  is the canon, or should explicit PATH become a lint alongside (c)?

## 5. Decision

Adopted 2026-08-04 (Mathijs) as recommended in v2:

- (a) IMPORT is universal across BUILDER/SERVICE/APP/ITEM.
- (b) LINK dissolves into store-aware COPY, **spike-first**: link-by-rule
  for store sources, with the two statically-known materialization
  triggers (role-dir/DIR mount beneath the destination; later assembly
  writes beneath). LINK survives as fallback if the spike sours; during
  the transition LINK parses with a deprecation hint toward COPY, and the
  regeneration sweep removes remaining uses before the alias is deleted
  (alpha rule: no long-lived compat).
- (c) The canon for executables is IMPORT + bare argv. Interpolated store
  paths in START/START_PRE stay legal; the lint lands with the
  regeneration sweep.
- (d) State binds at the app-native path via role dirs; v1's runtime-path
  LINK is rejected.

Open-question answers at adoption: top-level prelude IMPORT spill is
deferred (block-local first; revisit on regeneration evidence — Mathijs:
"maybe"); COPY union semantics are delegated to the spike; the explicit
`ENV PATH` question joins the (c) lint round.

## Changelog

- 2026-08-04: drafted (v1), revised (v2: STATEDIR-direct, LINK
  dissolution, single canon), and adopted, same day.

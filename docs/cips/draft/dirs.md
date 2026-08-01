# Dirs: declaration, materialization, and lifecycle

Status: draft, r3, 2026-08-01 — after Mathijs's second review round
(hermetic sharing, `DATADIR :ro`, reclassification, `.env`,
`cix recreate`) and the requested 4× turn-over (§5). Supersedes the
earlier binds.md and shared-rw.md drafts.

## 1. The problem

Three entangled gaps, one model. (a) *Operator host-binds*: 14 of 18
corpus compose files bind host paths into services (Immich media,
Paperless watch-dirs); composix has no spelling. (b) *Shared writable
surfaces*: Mastodon's web+sidekiq share one rw uploads dir; our edges are
sockets, role dirs are service-private — the one genuine edge-model gap
in the corpus. (c) *Lifecycle*: composix already has a richer persistence
ontology than docker — `STATEDIR`/`CACHEDIR`/`LOGDIR`/`RUNDIR` (D11)
each imply different durability — but what `cix down` (vs stop, vs
removal) does to each has never been spelled out, and "docker volume
prune" has no analogue.

## 2. Prior work

**Docker**: bind mounts (any host path, rw default, no declaration of
what the workload expects) and named volumes (objects with
create/rm/prune lifecycle, copy-up). One flat concept — "a volume" —
regardless of whether the data is cache, state, or logs; `volume prune`
exists precisely because nothing records which data is expendable.
`compose up --force-recreate` exists because containers accumulate
writable-layer state that only recreation clears.

**Kubernetes**: discourages `hostPath`; interposes PVC (the workload's
*claim*) bound to a PV (the operator's *location*) — need and location
split across the same seam as our manifest/compose. Sharing: `fsGroup`
chowns the volume to a declared gid and adds every container to it.
Ephemeral vs durable is per-volume-type (`emptyDir` vs PVC), chosen by
the workload author.

**systemd** natively encodes the lifecycle ontology docker lacks:
`RuntimeDirectory=` is *removed when the service stops*;
`StateDirectory=` persists; `CacheDirectory=` persists but is expendable
(`systemctl clean --what=cache`); `LogsDirectory=` has its own clean
class. `BindPaths=`/`BindReadOnlyPaths=` graft host paths into the
unit's namespace; `RequiresMountsFor=` orders units after their mounts;
idmapped mounts map a `DynamicUser` onto foreign ownership. Setgid dirs
+ `SupplementaryGroups=` + `UMask=0002` give fsGroup semantics; stable
identities/groups come from the D48(d) registry.

## 3. Recommendation

**The manifest declares dirs by role; compose decides materialization.**
Write-ness and lifetime come from the role, never from the bind entry.

Declaration: the existing D11 roles — `STATEDIR /var/lib/app`,
`CACHEDIR`, `LOGDIR`, `RUNDIR` — plus one new role from review:
**`DATADIR /media:ro`** — operator-supplied content the app reads (media
libraries, corpora); `:ro` is part of the declaration (an rw DATADIR is
the Paperless watch-dir case). A declared DATADIR makes `compose check`
*demand* the operator supply a materialization — it has no private
default (an empty private media dir is useless by construction).

The **role definitions are contracts**, written into docs as promises
("cix may delete CACHEDIR contents between runs at any time"): the
lifecycle table below is only as honest as these contracts, so they are
normative, not descriptive (see turn-over #1).

Materialization (compose, per declared dir; private is the default for
all roles except DATADIR):

- *(default)* **private**: the systemd directory, today's behavior.
- **`host: /tank/media`**: `BindPaths=`/`BindReadOnlyPaths=` per the
  role's write-ness. The host path must **pre-exist** — cix never
  creates directories outside its own roots; a missing path is a
  `compose check`/start error, and the unit gets `RequiresMountsFor=`
  so mount-dependent services order and fail honestly. Ownership: rw
  host dirs require a D48(d) identity; when existing data is owned
  otherwise, an idmapped mount maps the service identity onto it
  (turn-over #2). Compose env interpolation in paths
  (`${UPLOAD_LOCATION}`) is allowed.
- **`shared: <name>`**: a composite-owned surface joined by member
  services. **Hermetic** (review): every member must have *declared*
  the dir — an undeclared pack cannot meaningfully use a surface it
  never named. All members' declared roles for a shared surface must
  agree; disagreement is a check error (turn-over #3). v0 restricts
  sharing to `STATEDIR` and `DATADIR` — shared RUNDIR/LOGDIR have no
  corpus demand and RUNDIR's per-service stop-cleanup cannot ride a
  shared surface (turn-over #3). Mechanics: stable group from the
  registry, setgid dir, `SupplementaryGroups=` + `UMask=0002`.
- **`as: <role>` — reclassification** (review): compose may override
  the *treatment* of a declared dir. Escalating durability
  (CACHEDIR treated as STATEDIR) is silent — the operator buys more
  safety. Degrading durability (STATEDIR treated as CACHEDIR) is the
  loosening polarity: LOUD in check, since the operator is opting into
  data loss the pack did not sanction (D49a shape).

Extra, *undeclared* operator binds remain possible compose-side (ro
unless `write: true`) and are loud in check. `.env`: interpolation
values resolve at `cix up` time from the compose file's own directory
`.env` (that file only — no cwd ambiguity); resolved values enter the
generation identity so a changed `.env` restarts affected services;
secrets do NOT travel this road (env delivery is refused by the secrets
CIP — `.env` is for paths and ports; turn-over #4).

**Lifecycle table** (role contracts × events):

| event | RUNDIR | CACHEDIR | LOGDIR | STATEDIR | DATADIR | host-bound | shared |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `systemctl stop` / crash | removed | kept | kept | kept | untouched | untouched | kept |
| `cix down` | removed | kept | kept | kept | untouched | untouched | kept |
| `cix clean <svc> --what=…` | — | removable | opt-in | refused | refused | untouched | refused |
| composite removal + `--purge` | removed | removed | removed | removed | **never** | **never** | removed |

Purge confirms interactively with the exact path list (`--yes` for
automation). Host-bound and DATADIR content is operator property: cix
never deletes it, full stop. Nix GC is never involved.

**`cix recreate` dissolves** (review question): docker recreates
containers to shed writable-layer state; composix services have no
writable layer — restart already yields a pristine mount namespace, and
role dirs are *meant* to survive. The migration row reads
`compose up --force-recreate` → `cix up`/restart (nothing to recreate);
an operator who wants expendable state gone says so explicitly:
`cix clean <svc> --what=cache && systemctl restart …`. No recreate verb
— a fresh-start verb that implicitly deletes data classes would
contradict the table above (turn-over #4).

`cix run` grows the same materialization flags per CIP-77.

## 4. Open questions

1. ~~DATADIR~~ — in, spelled `DATADIR /media:ro` (review).
2. ~~hermetic sharing~~ — declared-only, plus role agreement (review).
3. ~~purge UX~~ — confirmed.
4. `.env` accepted with stated uncertainty — the resolution rules above
   (own-dir only, generation-identity inclusion, no secrets) are the
   proposed containment; flag anything that chafes.
5. New from turn-over #2: for pre-existing host data under a foreign
   uid, is the idmapped-mount mapping automatic, or does compose demand
   an explicit `owner:`-style acknowledgment before cix maps identity
   onto operator data? (Proposal: explicit acknowledgment — mapping
   silently onto foreign data is spooky.)

## 5. The 4× turn-over

1. **Role contracts vs lying packs.** The table's guarantees assume
   packs classify honestly; a pack that stores uploads under CACHEDIR
   turns `cix clean` into data loss *within contract*. Mitigations:
   role definitions published as normative contracts; reclassification
   (`as: state`) as the operator remedy; migration docs teach checking
   what upstream images actually store where (the wordpress
   entrypoint-surgery class).
2. **Host materialization imports docker's uid hell unless ownership is
   explicit.** BindPaths to a missing or foreign-owned path either
   fails at runtime or silently creates wrong ownership. Resolved:
   pre-existence required, `RequiresMountsFor=` ordering, D48(d)
   identity for rw, idmapped mounts for foreign-owned data — with the
   §4.5 question on how loud that mapping must be.
3. **Composed features contradict.** Shared×roles (members disagreeing
   on a surface's role), shared×RUNDIR (per-service stop-cleanup cannot
   govern a shared dir), shared×host×reclassify stacking. Resolved by
   restriction: role agreement enforced, sharing limited to
   STATEDIR/DATADIR in v0, and reclassification applies before
   sharing-legality is checked.
4. **New verbs and channels must not bypass the table.** `cix recreate`
   would be an implicit deleter — refused; deletion happens only
   through the two explicit verbs (`clean`, `--purge`). `.env` would be
   a side door for secrets into env — refused; and up-time resolution
   without generation-identity inclusion would silently deliver stale
   config — resolved by including resolved values in the identity.

## Changelog

- 2026-08-01: v1 as two drafts (binds, shared-rw); v2 merged with
  role/materialization split and lifecycle table; r3 — second review
  round absorbed (DATADIR `:ro`, hermetic sharing + role agreement,
  reclassification with polarity, `.env` containment, recreate
  dissolved) and the 4× turn-over recorded.

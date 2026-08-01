# Dirs: declaration, materialization, and lifecycle

Status: draft, 2026-08-01, v2 — supersedes the separate binds.md and
shared-rw.md drafts after Mathijs's review collapsed them: "a pack
doesn't care if [a declared dir] is materialized as a private thing or a
shared thing — composer concern", and the lifecycle of our
richer-than-docker dir ontology needs elucidation in the same breath.

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

**Kubernetes**: discourages `hostPath`; interposes PVC (the workload's
*claim*) bound to a PV (the operator's *location*) — need and location
split across the same seam as our manifest/compose. Sharing: `fsGroup`
chowns the volume to a declared gid and adds every container to it —
a declared shared group applied to data and writers. Ephemeral vs
durable is per-volume-type (`emptyDir` vs PVC), chosen by the workload
author.

**systemd** natively encodes the lifecycle ontology docker lacks:
`RuntimeDirectory=` is *removed when the service stops*;
`StateDirectory=` persists; `CacheDirectory=` persists but is expendable
(`systemctl clean --what=cache`); `LogsDirectory=` persists with its own
clean class. `systemctl clean` is the native "prune this service's
expendable data" with per-class selection. `BindPaths=`/
`BindReadOnlyPaths=` graft host paths into the unit's namespace and
compose with `DynamicUser` + idmapped mounts. Setgid dirs +
`SupplementaryGroups=` + `UMask=0002` give fsGroup semantics; per-edge
stable groups are the proven dstyle mechanism, and D48(d) already
decided: host-bound rw state needs a declared identity, shared
persistent edges need a stable group, both from the cix identity
registry.

## 3. Recommendation

**The manifest declares dirs by role; compose decides materialization.**
Write-ness and lifetime come from the role, never from the bind entry.

Declaration (exists today, D11): `STATEDIR /var/lib/app`,
`CACHEDIR /var/cache/app`, `LOGDIR`, `RUNDIR`. The pack says "I will
write here and it means state/cache/logs/runtime" — all systemd-speak —
and does not know or care how it is materialized.

Materialization (compose, per declared dir, default private):

- *(default)* **private**: the systemd directory, exactly today's
  behavior.
- **`host: /tank/media`**: `BindPaths=` to an operator path. Requires a
  declared identity (D48d) since a foreign filesystem must see stable
  ownership. Compose env-var interpolation in paths (`${UPLOAD_LOCATION}`,
  the Immich idiom) is allowed — resolved at `cix up` time.
- **`shared: <name>`**: a composite-owned surface joined by every
  service whose compose entry names it. Mechanics: stable group from the
  identity registry, setgid dir, members get `SupplementaryGroups=` +
  `UMask=0002`. Durability follows the declared role — Mastodon uploads
  are a shared STATEDIR (durable); a shared RUNDIR would be an ephemeral
  scratch handoff. This replaces the earlier "shared-dir edge" framing:
  sharing is a materialization of a declared dir, not a new object kind.

Extra, *undeclared* operator binds remain possible compose-side (ro
unless `write: true` — `ro: false` is wrong UX per review) and are the
loosening case: LOUD in `cix compose check` (D49a polarity). Read-only
content mounts (media libraries) are the expected use.

**Lifecycle table** (the elucidation):

| event | RUNDIR | CACHEDIR | LOGDIR | STATEDIR | host-bound | shared |
| --- | --- | --- | --- | --- | --- | --- |
| `systemctl stop` / crash | removed (systemd) | kept | kept | kept | untouched | kept |
| `cix down` | removed | kept | kept | kept | untouched | kept |
| `cix clean <svc> --what=cache,...` | — | removed | opt-in | refused* | untouched | refused* |
| composite removal + explicit `--purge` | removed | removed | removed | removed | **never touched** | removed |

\* state is never deleted by a cleaning verb; only the explicit purge
path deletes state, with confirmation listing exact paths. Host-bound
dirs are operator property: cix never deletes them, full stop. Nix GC is
never involved (role dirs are not store paths). `cix clean` is
`systemctl clean` sugar and inherits its per-class honesty.

**`cix run` is degenerate unary compose** (principle recorded in its own
draft CIP): run grows the same materialization flags with the same
vocabulary (`--dir state=host:/path`-shaped), no run-only concepts.

## 4. Open questions

1. Does the ro content mount deserve a *declared* role too (a pack that
   needs a media library could declare `DATADIR /media ro` so check can
   demand the operator supply it), or is undeclared-bind-plus-loud-check
   enough until a corpus case complains? (Jellyfin-class packs would
   benefit; proposal: defer, revisit at the Immich example.)
2. Shared dirs: must every member have *declared* the dir (slot-shaped,
   check-enforced), or may compose join a service that declared nothing
   (pure loosening, loud)? Proposal: declared-only — Mastodon's pack
   knows web and sidekiq both touch uploads.
3. `--purge` confirmation UX: interactive y/N with path listing, `--yes`
   for automation — enough?
4. Env interpolation in compose values: resolved from the invoking
   environment at `cix up`, or also from a `.env` file docker-style?

## Changelog

- 2026-08-01: v1 as two drafts (binds.md, shared-rw.md); v2 merged after
  review — role-declaration/materialization split, sharing as
  materialization not edge, lifecycle table added, `write: true` UX,
  compose env interpolation accepted, cix-run-as-unary-compose applied.

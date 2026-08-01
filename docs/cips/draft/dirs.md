# Dirs: declaration, materialization, and lifecycle

Status: draft, **r4**, 2026-08-01 — approved direction after three
dialogue rounds; ready for adoption review. Supersedes r3 (and the
original binds/shared-rw pair). Amends D11.

## 1. The problem

Three entangled gaps, one model. (a) *Operator host-binds*: 14 of 18
corpus compose files bind host paths into services (Immich media,
Paperless watch-dirs); composix has no spelling. (b) *Shared writable
surfaces*: Mastodon's web+sidekiq share one rw uploads dir; role dirs
are service-private by construction. (c) *Lifecycle*: composix has a
richer persistence ontology than docker (state/cache/logs/run), but
what `cix down` vs stop vs removal does to each was never spelled out.
Plus, from review: what does `STATEDIR /var/lib/postgresql` *mean*
mechanically, does it collide across services, and is the dir-family a
complexity monster?

## 2. Prior work

**systemd's directory classes** (`RuntimeDirectory=`, `StateDirectory=`,
`CacheDirectory=`, `LogsDirectory=`, `ConfigurationDirectory=`) are the
substrate ontology, and they do far more than clean-behavior: creation
(incl. parents; subpaths like `foo/bar` are documented), ownership —
with `DynamicUser` via the `/var/lib/private` symlink dance, recursive
re-chown on mismatch, and on current kernels **id-mapped mounts**
(host-side "nobody", service-uid in-namespace); `$STATE_DIRECTORY`-class
env vars; implied `BindPaths=` — *"when combined with RootDirectory=
these paths always reside on the host and are mounted from there into
the unit's namespace"* (free closed-root compatibility, systemd's own
words); per-class disposal (`RuntimeDirectory` removed at stop,
`systemctl clean --what=` for the rest); an optional `name:dest` symlink
alias (with a `:ro` flag since v257). Two structural facts: **names are
relative — arbitrary absolute locations are explicitly out of scope**
(*"a different mechanism has to be used"*), and there are exactly five
classes; systemd grows no sixth.

**Docker**: undifferentiated "volumes"; `volume prune` exists because
nothing records which data is expendable. **k8s**: the polarity
precedent — emptyDir/PVC (platform-provisioned) vs hostPath
(operator-supplied), `fsGroup` for shared writers. **FHS** `/srv` is the
nearest precedent for "operator-supplied data the service serves".
Nobody has a named DATADIR concept.

**Composix today** (crates/cix-run/src/unit.rs `add_directories`):
already unit-scopes the host side — `StateDirectory=<prefix>-<svc>` with
the symlink-alias to the declared destination when it lies under the
class root (the D11 restriction exists solely to enable that alias), a
`BindPaths=` fallback otherwise, `TemporaryFileSystem=/var/lib:ro`
masking, and `state-0/state-1` index names for multiple dirs.

## 3. Recommendation

### The model in one sentence

**Every dir declaration is a claim on the deployment; the dispositions
cix can satisfy itself — exactly systemd's classes — are satisfied
automatically and systemd-shaped; the one it cannot (operator-supplied
data) is an explicit `CLAIM data`.**

### Declarations (manifest)

- The role directives are frozen to systemd's classes: `STATEDIR`,
  `CACHEDIR`, `LOGDIR`, `RUNDIR` (+ config when configs land). A role
  dir is the **only route to durable private state** — by design.
  Declared paths are **arbitrary absolute in-namespace paths**
  (`LOGDIR /app/logs` is legal; the D11 conventional-root restriction
  is repealed — it only ever served the alias syntax). Multiple
  declarations per role stay legal.
- **`CLAIM data /media:ro`** (rw when writable, e.g. a Paperless
  consume dir) declares operator-supplied content: pre-existing, not
  service-owned, not cix-materializable, exempt from every deletion
  verb. `compose check` *demands* a materialization for it — there is
  no private default (an empty private media dir is useless by
  construction).
- **DATADIR is deliberately rejected**: dressing the operator-supplied
  inverse in the *DIR family's clothes is the CIP-80 CMD-lesson —
  borrowing a family's look without its contract teaches exactly the
  wrong intuition (every *DIR is cix-created and service-owned; this
  one is neither). The noun `data` lives on inside the claim. (`CLAIM
  mount` was rejected for colliding with the existing manifest `mounts`
  field, the D22 item projections.)

### Backing (the overlay shape)

Private materialization uses one host root per unit per class, with the
**full declared path mirrored beneath it**:

```
LOGDIR /app/logs   →  LogsDirectory=cix-<comp>-<svc>/app/logs
                      host: /var/log/cix-<comp>-<svc>/app/logs
                      + BindPaths= to /app/logs in the namespace
```

systemd still does everything (creation, id-mapped ownership, clean
classes — subpath directives are documented); collision-freedom comes
from the unit-scoped root (two postgres services = two host roots, each
privately seeing its own declared paths — what scenario-side-by-side
proves). The alias branch, the D11 restriction, and the `state-N` index
names are all deleted; host paths self-describe for operators. cix
overrides the `$*_DIRECTORY` env vars to the declared in-namespace
paths (systemd would name the host-side locations).

### Materialization (compose, per declared dir)

| | backing | machinery active | identity |
| --- | --- | --- | --- |
| *(default)* private | unit-scoped host root, path-mirrored | full systemd: creation, id-mapped/DynamicUser ownership, env, clean | dynamic OK |
| `host: /tank/x` | operator path, `BindPaths=`/`BindReadOnlyPaths=` per role write-ness | none — bind + `RequiresMountsFor=`; path must pre-exist, cix never mkdirs outside its roots | **static (D48d) required** — nobody re-chowns operator data |
| `shared: <name>` | composite-owned surface (v0: STATEDIR and CLAIM-data only) | stable group + setgid + `SupplementaryGroups=` + `UMask=0002` | registry group |
| `as: <role>` | reclassification of treatment | target role's | — |

Reclassification polarity: escalating durability (cache→state) is
silent; degrading (state→cache) is LOUD in check (operator opts into
data loss the pack did not sanction, D49a shape). Shared surfaces are
hermetic: every member must have declared the dir, and members' roles
must agree — disagreement is a check error. Undeclared extra operator
binds stay possible (ro unless `write: true`), loud in check.

`.env` containment: interpolation resolves from the compose file's own
directory `.env` only, resolved values enter the generation identity
(changed `.env` ⇒ restart-changed sees it), and secrets never travel
this road (CIP-81 refuses env delivery).

### Lifecycle table (role contracts × events)

| event | RUNDIR | CACHEDIR | LOGDIR | STATEDIR | CLAIM data | host-backed | shared |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `systemctl stop` / crash | removed | kept | kept | kept | untouched | untouched | kept |
| `cix down` | removed | kept | kept | kept | untouched | untouched | kept |
| `cix clean --what=…` | — | removable | opt-in | refused | refused | untouched | refused |
| composite removal + `--purge` | removed | removed | removed | removed | **never** | **never** | removed |

Purge confirms interactively with exact paths (`--yes` for automation).
Role definitions are normative contracts in the docs ("cix may delete
CACHEDIR contents between runs") — the table is only as honest as pack
authors' classifications; reclassification is the operator remedy.

**`cix recreate` dissolves**: docker recreates containers to shed
writable-layer state; we have no writable layer — restart yields a
pristine namespace and role dirs are *meant* to survive. Migration row:
`compose up --force-recreate` → `cix up` (nothing to recreate);
explicit expendable-state removal is `cix clean`. No implicit-deletion
verbs, ever.

`cix run` grows the same materialization flags (CIP-77).

## 4. Open questions

1. **Host-mirror spelling**: full mirror (`STATEDIR /var/lib/pg` →
   host `/var/lib/<unit>/var/lib/pg`, ugly-but-uniform) vs stripping
   the class prefix when the declared path happens to lie under it
   (`/var/lib/<unit>/pg`, pretty-but-conditional). Proposal: full
   mirror — host layout is cix-internal, uniformity wins.
2. **Foreign-owned host data**: for `host:` backing over data owned by
   another uid, is the idmapped-mount mapping automatic or does compose
   demand an explicit acknowledgment field before cix maps the service
   identity onto operator data? Proposal: explicit — silent mapping
   onto foreign data is spooky.
3. `.env`: accepted with stated uncertainty — flag anything that chafes
   in the containment rules above.

## 5. The 4× turn-overs (r3 + r4, recorded)

r3: lying packs (→ normative contracts + reclassification); docker's
uid hell at the host seam (→ pre-existence + static identity + idmap);
feature stacking (→ role agreement, sharing restricted, reclassify
before legality); no verbs/channels bypass the table (→ recreate
refused, .env contained). r4 (the overlay round): systemd machinery
survives arbitrary paths via subpath directives; unit-scoped roots keep
collision-freedom; the claim unification holds — and DATADIR's contract
inversion (not private, not created, must pre-exist) is exactly what
makes it a claim, not a dir.

## Changelog

- 2026-08-01: v1 (binds + shared-rw), v2 (merged, role/materialization
  split), r3 (review round: DATADIR:ro, hermetic sharing,
  reclassification, .env, recreate; 4× turn-over). r4 (dialogue rounds
  on mechanics + aesthetics): claim unification as the model, overlay
  backing kills the D11 restriction/alias/indices, machinery table,
  DATADIR rejected on the CMD-lesson → `CLAIM data`, id-mapped-mount
  and closed-root notes from the systemd docs.

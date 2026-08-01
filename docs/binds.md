# Operator host-binds and the volume question

Status: proposal, 2026-08-01. Decision pending.

## 1. The problem

Real stacks graft host paths into services: 14 of the 18 surveyed compose
files use bind mounts (docs/corpus.md), from env-interpolated media paths
(Immich) to consume-watch directories (Paperless-ngx). Composix today has
role dirs (systemd-managed service-private state, D11) and read-only item
projections (D22) — there is no way for an *operator* to say "this
service sees `/tank/media` at `/media`". The ledger defers it as
"⏳ compose era, operator territory" (docs/docker.md); the compose era is
now. Adjacent and entangled: does composix ever grow docker's named
*volume object* (create/inspect/ls/rm lifecycle), and what is the cleanup
story for mutable data (`volume prune` has no safe analogue — Nix GC
never touches role dirs)?

## 2. Prior work

**Docker bind mounts** are maximally permissive — any host path, rw by
default — and famously messy: uid mismatches between container and host
users, SELinux `:z` relabeling, and no declaration of what the workload
expects; the mount is pure operator assertion. **Docker named volumes**
wrap mutable data in an object with lifecycle (`volume create/rm/prune`),
driver plugins, and copy-up-on-first-mount semantics. The wild uses both
heavily, but treats volumes mostly as "durable dir I don't name a path
for" — the object-ness (drivers, cross-service attachment) is rarer.

**Kubernetes** discourages `hostPath` outright and interposes an
indirection: the pod declares a *claim* (PVC — "I need 10Gi RWO"), the
cluster binds it to a *volume* (PV — the actual location), and only the
claim appears in the workload spec. The lesson for us is the split
itself: **the app declares the need; the operator supplies the location**
— the same polarity split D49(a) records for egress (need = app
knowledge, usage = instance knowledge).

**systemd** has the mechanism natively: `BindPaths=` /
`BindReadOnlyPaths=` on a unit graft host paths into the service's mount
namespace, composing with `DynamicUser` and idmapped mounts. Ownership of
host-bound data by a dynamic user is exactly the problem D48(d) already
decided: host-bound state requires a declared identity from the
cix-managed registry (`--user` mode dissolves it — everything is the
invoking user).

## 3. Recommendation

**No volume objects, ever.** Role dirs + binds cover the corpus; the
object layer (drivers, copy-up, volume CLI) is docker plumbing that
dissolves. Record as ❌ in the ledger rows for `volume create/...` and
volume drivers.

**Operator binds are compose-only** (never `cix run` flags in v0, never
manifest-side paths): per-service
`binds: { "/tank/media": { at: "/media", ro: true } }` in compose,
compiled to `BindReadOnlyPaths=`/`BindPaths=`. Manifest-side, the app MAY
declare an expected mount point (an *external dir* slot, name + path —
PVC-shaped), and compose binds a host path to the slot. Undeclared binds
are the loosening case and therefore LOUD in `cix compose check` (D49a
polarity: granting what the artifact did not ask for is said out loud);
declared-but-unbound slots fail check.

**Ownership** rides D48(d): a service with rw host-binds must have a
declared identity; ro binds work with `DynamicUser` as-is. **Lifecycle:**
bound host paths are operator property — cix never deletes them; role
dirs keep `systemctl clean` semantics. A `cix prune`-style inventory
("which units own which role dirs, what would deletion free") is CLI
sugar for later, not an object model.

## 4. Open questions

1. Manifest slot: is the declared *external dir* (name + mount point,
   ro/rw) worth manifest schema now, or do we ship compose-side binds
   first and add the declaration when `compose check` loudness proves
   insufficient?
2. rw default: propose ro-by-default (`ro: false` to opt in) — docker's
   rw default is the wrong default. Agreed?
3. Env-interpolated bind paths (Immich's `${UPLOAD_LOCATION}`): compose
   is canonical JSON (D28) — do we allow `${ENV}` interpolation in
   compose values at all, or is that the operator's templating problem?
4. The Paperless watch-dir (host-shared rw, another process also writes):
   any extra semantics needed beyond a rw bind + D48d identity, or
   document-and-done?

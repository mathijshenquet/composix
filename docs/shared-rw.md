# A shared writable surface between services

Status: proposal, 2026-08-01. Decision pending. The one genuine
edge-model gap in the corpus (docs/corpus.md §4.3).

## 1. The problem

Mastodon's `web` and `sidekiq` share one rw uploads directory; Penpot's
services share an assets volume. Composix edges today are
producer→consumer sockets and read-only paths — there is no way for two
services to *both write* one durable directory. Role dirs (D11) are
service-private by construction. This blocks the corpus's top example
candidate (the Mastodon-shaped stack) and any app whose workers and web
tier exchange files through the filesystem.

## 2. Prior work

**Docker**: mount the same named volume in both services. It works
because everything runs as the same uid or the app chowns at startup —
no ownership semantics at all; collisions and permission bugs are the
user's problem.

**Kubernetes** has two answers. Co-located: containers in one *pod*
share an `emptyDir`/PVC — the pod is the sharing unit, and sharing is
trivially safe because colocation is explicit. Cross-workload: an RWX
PersistentVolumeClaim mounted by several pods, with `fsGroup` in the pod
security context — kubelet chowns/chmods the volume to a declared gid
and adds the containers to it. `fsGroup` is the load-bearing mechanism:
**a declared shared group, applied to the data and to every writer**.

**dstyle** (our own prior work, proven in the compose examples): unix
edges get a *per-edge group* — membership in the group is the grant.
That mechanism extends naturally from "may connect to this socket" to
"may write this tree".

**POSIX/systemd**: a setgid directory + a stable group +
`SupplementaryGroups=` per writer gives exactly k8s-fsGroup semantics;
`UMask=0002` makes group-writability stick for new files. Stable groups
for shared persistent data are already decided territory: D48(d) — shared
persistent edges require a stable group from the cix identity registry.

## 3. Recommendation

Model it as an **edge variant**, not a volume: a `shared-dir` edge in
compose declares a named writable surface with an explicit member set.
Mechanics per D48(d) + dstyle: the edge owns a stable group from the
identity registry; the data lives in a role-dir-shaped state directory
*owned by the edge* (not by either service — members come and go, the
edge persists); the directory is setgid + group-rw; every member service
gets `SupplementaryGroups=<edge-group>` and `UMask=0002`. Manifest-side,
a service that needs a shared surface declares the mount-point slot (same
slot shape as docs/binds.md external dirs); compose wires slots to the
edge. Symmetric membership (no producer/consumer polarity — writers are
writers), consistent with how the wild uses it.

The k8s pod answer (colocate and share) is NOT the substitute: Mastodon's
web and sidekiq are genuinely separate services (own scaling, own
restarts); forcing them into one D43 pod to share files would misuse
colocation for a storage grant.

Explicitly out: cross-composite sharing (an edge lives inside one
composite), quotas, and any file-event delivery (writers coordinate
through the app's own queue — Mastodon already does).

## 4. Open questions

1. Is setgid+umask honest enough, or do hostile-umask apps (files created
   0600) force idmapped-mount ACL machinery in v1? Proposal: ship
   setgid+umask, document the failure mode, escalate on real corpus
   evidence.
2. Lifecycle: `cix down` keeps the edge's data (like state dirs);
   deleting it needs an explicit operation. `cix compose rm-edge`-class
   surface now or later?
3. Naming: `shared-dir` edge vs making it a property of the existing
   edge object (`kind: dir` next to `kind: unix`)? Proposal: same edge
   object, new kind — one grant vocabulary.

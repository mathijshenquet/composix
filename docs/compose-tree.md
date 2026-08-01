# The compose tree — one grammar from build output to host

Status: **decided 2026-07-30** (design round with Mathijs; D-numbers recorded in
docs/design.md D40–D46, which win on conflict). This paper is the connected story;
docs/compose-netns.md remains the *realization* paper for the netns mechanics and is
amended by this round (naming + optionality; see below).

## The thesis, one sentence

Everything is **nix store + a handful of named, moving pointers + systemd**: a name in
the index is a mutable pointer to a content-addressed tag table; a host root is a mutable
file pointing at composite refs; a generation is a pointer to a built tree — three small,
diffable, owned mutable cells, and everything else immutable and content-addressed.

## 1. Two artifact kinds, one tree grammar

- A **pack item** is a store item: filesystem + `cix-manifest.json`, where the manifest
  is a single bare **def-node** — *one item = exactly one service* (D41). No services
  map. The def-node carries what the software needs: `exec`, `setup`, typed `env`
  declarations, `ports`, `listeners`, `dirs`, `health`, `jit`, `egress`.
- A **compose artifact** is a store item: `cix.json` (+ advisory lock snapshot), a
  **group-node**: children (ref-nodes), `edges`, `publish`, `network`, nested groups.
- Every level of the system — a leaf's manifest, a published composite, the host root —
  is a node in the *same* tree grammar. The manifest is the leaf body; compose is
  everything above it.

Rationale for single-service items: granular rebuilds (one tag per service, track on the
right grain), and the store dedupes shared closures at path level so splitting costs zero
bytes. The multi-service manifest was a pre-compose-era artifact (D8) and is retired;
the `service:` selector field in compose dies with it.

## 2. The tree

```json
{
  "cixCompose": 1,
  "name": "projA",
  "network": "pod",
  "children": {
    "api":    { "item": "projA-api:v7" },
    "worker": { "item": "projA-worker:v7" },
    "db":     { "item": "pg:v16" },
    "batch":  { "compose": "projA-batch:v7" }
  },
  "edges": {
    "database": { "producer": { "child": "db", "path": "/run/pg" },
                  "consumers": { "api": {}, "worker": {} } }
  },
  "publish": { "http": { "child": "api", "port": "http" } }
}
```

- **One `children` map** (2026-07-30 review, Mathijs): no services/composites split —
  pod-ness gives every child, leaf or node, a well-defined surface, so the parent
  never needs to know the kind; `"item":` vs `"compose":` (or an inline subtree)
  discriminates inside the entry. Same per-child options everywhere: env overrides,
  `bind`, update policy. Version keys stay per document kind: `cixManifest` on leaf
  manifests, `cixCompose` on tree files — no shared counter, no bare `cix` key.
- **Refs are always fully qualified** (`name:tag`, never bare); `cix publish`
  additionally REJECTS refs that are ambiguous outside their origin — local-only
  refs are a deploy-time convenience, never published artifact content.
- **Instance identity = the path in the tree**, not the artifact's self-`name`. Unit
  `cix-<path…>-<svc>.service`, slice `cix-<path…>.slice` (nested; the resource axis is a
  real tree), state dirs keyed by path. Two instances of the same artifact under
  different keys are fully disjoint (prod + staging of one tag on one host).
- The **host root** is the same format in a mutable file (`/etc/cix/cix.json` + lock):
  the one file an operator edits. CLI day-two verbs (`cix compose add <ref>` …) are
  *structured edits of that file* — the D28 machine-format payoff. Imperative and
  declarative do not compete when the CLI's target is the file: the slob is diffable,
  git-able, owned.

## 3. Pod-ness (D43)

`network: "pod"` is a **property a composite may claim**, not a fixed stratum:

- A pod = one netns for its subtree: shared loopback, intra-pod addressing is
  `localhost:<port>`, per-pod port collision freedom.
- **Nearest-pod-ancestor wins**: each service lives in the netns of its nearest ancestor
  that claims pod-ness. Pod-in-pod is legal — the child keeps its own namespace inside
  the parent's *scope*; embedding never strips a sealed artifact's declared boundary.
- Crossing a pod boundary requires declaration: `publish` (upward, one boundary at a
  time — a child's publish surfaces into the parent's scope, where the parent `bind`s it
  or re-publishes further up), `edges` (fd capability tier, crosses netns for free, D25),
  or later named networks (D26).
- **Absence of pod-ness anywhere = today's honest host networking** (rawdogging is the
  absence of a property, not an escape flag). Trusted-world users write nothing.
- Per-service **`egress: true`** (spec-level, D20 app semantics) declares outward
  initiation. Only enforceable under a pod ancestor; a loud no-op
  otherwise. Absence = loopback-only view even when the pod has a veth — and a composite
  with no egress needs gets **zero machinery** (no veth, no routes).
- **A pod member is type-identical to a service member** from the parent's view: every
  child is a name with surfaces (ports/listeners for a service, publishes for a pod), an
  egress need (declared, or bubbled up from inside the pod), and a resource footprint —
  the wiring algebra and the repin interface-check work on that type uniformly. The
  honest asymmetry sits one level down: a service member shares its scope's loopback
  (siblings reach it on *any* port — deliberately ambient inside the trust boundary),
  a pod member is opaque (published surfaces only). Consequence: **wrapping a service in
  a pod is a local, non-breaking tightening** — keep the surface names and no parent
  wiring changes; promote-to-pod (and its inverse) is a per-node knob, not a redesign.
- The k8s mapping, honestly: composite-with-pod-ness ≈ pod (shared netns, sidecars);
  slice tree ≈ node cgroup hierarchy (`cix.slice` ≈ `kubepods.slice`); D26 networks ≈
  CNI; D27 talks-to ≈ NetworkPolicy; reconciler ≈ kubelet. Deliberate deviation: our
  "pod" may be trust-boundary-sized, not sidecar-sized — the grain is the user's choice,
  and choosing it selects capability-tier wiring (inside) vs network-tier (between).

## 3b. Edges, precisely — and the co-location constraint

An edge is a **directed capability claim of a unix-socket surface**, nothing more.
The producer declares the runtime dir its socket lives in; the edge compiles to a
per-edge group, that dir group-owned, and ONLY the consumers receiving that group
membership plus the dir made visible in their namespaces (at a path of their
choosing). Authorization = possession of the path (fs permissions + `SO_PEERCRED`) —
the D25 capability model: no IP, no discovery, no firewall rules; a non-member does
not even see the socket. In the tree, a producer may also be a child composite's
published surface. The wiring graph is therefore literally
"who may reach whose socket".

**Filesystem sharing implies co-location** (2026-07-30 review, Mathijs — the k8s
RWO/RWX question): everything fs-based (unix edges, shared-rw dirs) only works with
members on one host. Rule, recorded now to pre-commit multi-host correctly:
*fs-tier edges constrain their members to co-location; the pod is the atomic
co-location unit; a tree may later be split across hosts only along IP-tier
connections (D26) — never through an fs edge.* Today (single host) trivially
satisfied; tomorrow it is the placement constraint a multi-host realization must
honor — exactly k8s's "containers sharing volumes live in one pod; cross-node
sharing needs RWX storage or the network".

## 4. Refs and locks (D44) — the same rule on every floor

- A ref is **always `name:tag`**. No bare names, no version ranges, no solver. Tags are
  moving pointers by nature; how soft a tag is, is the publisher's tag discipline
  (immutable release tags or a hash-qualified ref when you want hard sealing).
- **The operative lock lives with whoever deploys.** A published composite embeds an
  *advisory* lock snapshot ("what the author resolved/tested against") used to seed the
  deployer's first resolve and for hermetic testing — it is provenance, not authority.
- **Update = deliberate repin**: `cix up` replays the lock; `cix up --update [edge…]`
  re-resolves named edges anywhere in the tree and pins them in the host lock. Root-side
  policy `track` on an edge = auto-repin on every up (the reconciler case). There is no
  publisher-side pin/track anymore.
- Every generation is fully pinned — reproducible and rollbackable regardless of how
  much tracking happens. The sealed/tracking axis only decides *who may move which
  pointer*, never whether a deployment is pinned.
- **Repin check**: wiring-as-interface. When a tag moves, `cix compose check` verifies
  the new artifact still provides the surfaces the tree uses of it (publishes, edges,
  ports). Shape-matching on existing declarations; no constraint algebra.
- **Override** (deployer moves an edge against the publisher) is deliberately NOT built:
  evidence-gated future, cargo-`[patch]` shape — declared at the piercing level, in that
  level's own file, visible in diff. Unit-property piercing already exists natively as
  systemd drop-ins + `systemd-delta`.
- Rollback semantics: generations are the mechanical crash net (previous known-good
  combination, atomic); *semantic* undo is pushing/repinning a tag — roll-forward, like
  k8s `rollout undo` secretly is. Honest gaps shared with docker/k8s, stated: rollback
  restores units, not data; and it only catches detectable failure.

This is flake-input semantics re-derived at the compose level — the same move D32 made
at the Cixfile level. Cixfile FROM, compose refs, host root: **refs move, locks pin,
rolling is a deliberate act at the level that deploys.**

## 5. The index re-founded (D45)

Today: one JSON sidecar per tag + a GC-root symlink per ref. Re-founding:

- Per **name**, one content-addressed store item holding the **tag table**
  (`tag → {storePath, narHash, meta}`) plus the **parent table's hash** (history chain).
- The only mutable cell: a tiny `name → table-item-hash` map. Publish = build new table
  item, **CAS the name pointer** (expected-old → new; concurrent publishers detect
  races). Multi-tag publishes are atomic per name.
- Yank = publish a table without the tag: fresh resolves stop offering it; existing
  locks/copies keep working (advisory, crates.io-style — content-addressed bytes cannot
  be recalled and we don't pretend). Real deletion stays GC of store items (D35:
  lifecycle = tags + GC). Pointer staleness is a TTL concern on one lookup.
- Signing (D35 ⏳ publish-era) collapses: **sign the table hash per name**; name
  ownership = key. **Auth is name-level: "who may move this name."** A transparency log
  is a later small step — the hash chain already exists.
- Serving gets dumber: bytes via substituters, the index HTTP layer degrades toward a
  static `name → hash` lookup.

Prior art convergence: git (refs → objects), OCI (tag → content-addressed manifest),
crates.io (per-name index files), nix channels (name → atomic world snapshot).

## 6. Computable composes (D46)

A **parametric compose** makes a tag family computable: `my-app:$tag` ≡ compose of
`my-frontend:$tag` + `my-backend:$tag`. Rules:

- `$tag` is the **only** variable, usable only in the tag position of refs.
- Expansion happens at **publish time**, never at resolve time: `cix publish my-app:v1.2.5`
  substitutes, materializes an ordinary concrete compose artifact, updates the tag table.
  Resolvers stay dumb data readers (no interpreter in the index — the Helm disease stays
  outside).
- Monorepo tooling on top: one verb publishes the whole family for a version bump —
  build all items, tag `my-frontend:v1.2.5 … my-app:v1.2.5`, one atomic table move per
  name. ("I update my monorepo to v1.2.5 and the whole bunch is published.")

## 7. Prior work (schools and what they taught us)

- **k8s core**: merged everything into one API schema with no sealed intermediates → the
  market rebuilt layering as text templating (Helm/Kustomize). Lesson: bake the floors
  into the model.
- **Helm**: distributable chart + Chart.lock, but values pierce everything from the top —
  templates, not artifacts; weak reproducibility. **Kustomize/NixOS modules**: piercing
  as the product; provenance suffers. Lesson: pierce-by-default destroys sealing.
- **Terraform modules**: interface-only school — strong contracts, knob inflation.
  Lesson: you need a declared-escape next to the neat door.
- **Cargo/npm/bzlmod/flakes**: lock-per-level with explicit root-declared overrides —
  the school we're in.
- **GitOps (Flux/Argo)**: root-as-file + reconciler tracking refs; Argo app-of-apps is
  the closest prior art for the composite tree; Flux image-automation proves everyone
  rebuilds "tags move, locks pin" in CI — we make it first-class.
- **systemd**: portable services (sealed image + units); drop-ins + `systemd-delta` =
  unit-level override with provenance, already in our substrate.
- What none of them have: the whole chain — build output to host root — as one
  content-addressed tree grammar with every floor taggable in an index. That gap is
  composix.

## Appendix: the bittensor-bot fleet as a worked case

A real fleet (ohio3/beast/dev): per host ~20–24 numbered subtensor nodes (each one
container: own mem limits, per-index volumes `/data/subtensorN`, indexed ports
`1001+N…`), plus a bot, a dashboard, a dataplatform, watched by an on-box `fleetd`
(6s reconcile loop, rolling updates, health watermarks) driven from one `config.py` →
generated per-host compose files + `target-state.json`.

Mapping onto the tree: node = leaf composite **with pod-ness** (one container per pod);
the node set = an intermediate composite *without* pod-ness contributing policy (rolling
update, budgets) — a StatefulSet-shape (numbered identities, per-index state), which is
where D30's deferred scale/replicas belongs (`replicas` as a node property); per-host
instantiation = the host root parameterized per host; `fleetd` = the reconciler;
`target-state.json` = the host root file; the `1001+N` port arithmetic mostly dissolves
(each node binds its natural RPC port in its own netns; only genuinely
outside-reachable p2p ports keep indexed publishes at the root).

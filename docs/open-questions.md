# Open questions inventory — migration docs & compat ledgers

Status: working inventory, swept 2026-08-01. Sources: every ❓/⏳ row in
docs/docker.md, the gap rows in docs/migrate.md, docs/corpus.md §4, and
the deferral notes in docs/design.md. Decisions land in design.md as
D-numbers; this doc routes each open item to either a design doc, a
proposed one-line disposition (awaiting Mathijs), or an era.

## Clustered into CIP drafts (docs/cips/draft/, reviewed 2026-08-01)

| Draft | Covers | Ledger rows absorbed |
| --- | --- | --- |
| [health](cips/draft/health.md) | liveness/readiness, health-graph ban | `HEALTHCHECK` (docker.md, migrate.md), corpus demand #1 |
| [dirs](cips/draft/dirs.md) | dir declaration/materialization/lifecycle: operator host-binds, shared writable surfaces, volume-object refusal, down/clean/purge semantics (merges the earlier binds + shared-rw drafts) | `-v/--mount`, named volumes, `volume create/ls/rm/prune/update`, bind mounts, corpus demands #2+#3 |
| [timers](cips/draft/timers.md) | compose `schedule:` on raw OnCalendar | corpus demand #4, Renovate row |
| [secrets](cips/draft/secrets.md) | runtime `SECRET`/LoadCredential + build-time fetch credentials | compose secrets ⏳, BuildKit secret/SSH mounts ❓, migrate.md build-secrets ❌, `docker pass` ❓ (host keychain: out of scope), CLI credential stores ❓ (the fetch-credential file), corpus demand #8 |
| [devices](cips/0078-devices.md) | `CLAIM gpu`, `SHM`, tmpfs sizing | `--device/--gpus` ⏳, `--shm-size` ⏳, `--group-add` ❓ (dissolves into claims), tmpfs mounts ❓, compose-level `--privileged` override ❓ (stays ❌) |
| [devloop](cips/draft/devloop.md) | `cix watch` = rebuild+restart; sync refused | compose `watch` ❓, live-reload dev binds |
| [run-unary-compose](cips/draft/run-unary-compose.md) | design invariant: `cix run` = compose with one anonymous member | the recurring "and what about cix run?" question |

Bikeshed backlog (small, self-contained): `START` naming — prior art
docker `ENTRYPOINT`/`CMD`, k8s `command`/`args`, systemd `ExecStart=`,
Procfile; current recommendation: keep `START` (exec(2) honesty, D55).
Probe directive spelling: `READINESS`/`LIVENESS` proposed (health §4.1).

## Already decided — propose recording the disposition in the ledger

- **`LABEL` / display metadata** → D54 (annotations designed,
  deliberately unbuilt). migrate.md's "open gap" wording should cite D54.
- **Migrate-on-upgrade hooks** → D48(f) (ordinary oneshot + ordering;
  abort-before-switch deferred).
- **Fixed uid/gid workloads** (`USER` row) → D48(d) identity registry.
- **`ARG` / build args** ❓ → covered by D32 (generated Cixfile text is
  the parameter channel) + D46 (parametric composes, publish-time
  expansion). Propose re-marking 🔁 with those citations. The gitea
  version-stamp case gets a documented pattern (stamp file COPY'd in),
  not a mechanism.
- **`SHELL`/`ONBUILD`/parser directives, `FROM` arbitrary base images,
  docker.sock, commit, privileged, plugins** — already coherent ❌ class;
  no action.

## Proposed one-line dispositions (cheap to bless or overrule)

- **`docker cp`** ❓ → ❌ + docs: role dirs are host paths; `cix inspect`
  names them. Copying into the immutable item is anti-model.
- **`container prune` / cleanup contract** ❓ → fold into compose
  lifecycle when demanded; transient units already collect (D63).
- **`--name` stable handle** ❓ → compose provides stable names
  (`cix-<comp>-<svc>`); a `cix run --name` is trivial sugar, ⏳ until
  asked.
- **`STOPSIGNAL` / stop timeouts** ⏳ → mechanical manifest fields
  (`KillSignal=`, `TimeoutStopSec=`); no design needed, schedule as a
  small track when a corpus app needs it.
- **`cix logs` / `stats` / `system df` / `prune` sugar** ❓ → one
  "observability & janitor CLI" pass post-compose-v1; journald/cgtop are
  honest interim answers. Not design-heavy; needs a taste round on
  selectors.
- **Namespace sharing modes (`--ipc/--pid/--uts`)** ⏳❓ → D43 pods are
  the sharing unit; standalone sharing flags stay ❌.
- **Restart policies** ⏳ → compose maps to `Restart=`/`RestartSec`
  fields; mechanical, rides the health round (LIVE = opt-in restart).
- **`docker init` generator** ❓ → the migrate prompt (docs/migrate.md)
  IS the current generator; a `cix init` skeleton is tooling-era ⏳.
- **Docker Offload** ❓ → ❌; nix remote builders are the delegation
  story.
- **AppArmor/SELinux labeling** ❓ → out of manifest scope; document
  as host policy, revisit on a real SELinux-host user.
- **Desktop Enhanced Container Isolation** ❓ → ❌; different threat
  model (VM boundary), out of thesis.
- **Authorization plugins / policy point** ❓ → deferred to the
  server/reconciler era (D9); no plugin interface ever.
- **Engine API / SDKs** ❓ → reconciler era (D9).
- **Remote contexts / `--host`** ❓ → ssh is the transport; sugar ⏳.
- **`docker mcp`** ❓ → ❌ irrelevant to the runtime; ecosystem tooling
  can sit on `cix` CLI output.
- **Capabilities coverage beyond NET_BIND_SERVICE** ❓ → grow
  capability-by-capability with dogfood (devices.md mints the next two); never a
  raw `--cap-add`.
- **userns honesty note** ❓ → keep the ledger's honesty wording; a
  hardening comparison doc is worthwhile when security review era
  starts.

## Era-parked (decided deferrals, no action now)

- **Publish era** (D17/D35): push, login/auth, entry signing, mirror
  redistribution, pull-through fill, hub/orgs/search/webhooks, SBOM &
  attestation exchange formats (+ Scout-class analysis on closures).
- **Networking era** (D26/D27/D49): bridge/DNS, finer isolation,
  network objects/drivers, `--hostname/--dns/--ip`, per-netns sysctls,
  port remapping/NAT & bind-address policy.
- **Compose v1+** (D30 ledger): replicas/scale, resource limits (slice
  properties), configs objects, reusable top-level objects, live
  `update`.

## Errors found during the sweep

- docs/docker.md row "registry mirrors" appears twice (§index and
  §distribution) with different dispositions (🔁 D35 vs 🔁 D6 + ❓) —
  needs merging into one row citing D35.
- docs/migrate.md `LABEL` row says "record display metadata as an open
  gap" — stale: D54 decided it; should cite D54.
- docs/corpus.md §4.1 still calls health "the most-demanded deferral →
  schedule early" without pointing at the now-existing proposal — add a
  health.md pointer once that decision lands.

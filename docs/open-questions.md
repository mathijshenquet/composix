# Open questions inventory — migration docs & compat ledgers

Status: swept 2026-08-01; cleaned same day after the CIP wave. Sources:
every ❓/⏳ row in docs/docker.md, the gap rows in docs/migrate.md,
docs/corpus.md §4, and the deferral notes in docs/design.md. New
decisions land as CIPs (docs/cips/); this doc tracks what remains
genuinely open.

## Resolved into adopted CIPs (2026-08-01)

| CIP | Decided | Absorbed |
| --- | --- | --- |
| [CIP-75](cips/0075-timers.md) | compose `schedule:` on raw OnCalendar | corpus demand #4, cron/scheduler sidecars |
| [CIP-76](cips/0076-devloop.md) | `cix watch`; sync ❌ forever | compose `watch` ❓, live-reload binds |
| [CIP-77](cips/0077-run-unary-compose.md) | `cix run` = unary compose invariant | the recurring "what about cix run?" |
| [CIP-78](cips/0078-devices.md) | `CLAIM` vocabulary; `CLAIM gpu`/`device`, `SHM` | `--device/--gpus` ⏳, `--shm-size` ⏳, `--group-add` ❓, tmpfs ❓, compose `--privileged` ❓ (stays ❌) |
| [CIP-79](cips/0079-health.md) | READINESS/LIVENESS; health graph banned | `HEALTHCHECK` rows, corpus demand #1 |
| [CIP-80](cips/0080-exec-naming.md) | EXEC→START, SETUP→START_PRE | the EXEC bikeshed |
| [CIP-81](cips/0081-secrets.md) | SECRET/LoadCredential; fetch tokens + host-side consent | secrets rows, BuildKit secret mounts ❓, `docker pass` ❓, credential stores ❓, corpus demand #8 |
| [CIP-82](cips/0082-dirs.md) | dirs: claims model, overlay backing, `DIR`, lifecycle table | `-v/--mount`, named volumes + volume CLI ❓, bind mounts ⏳, `volume prune`/mutable-GC ❓, `container prune` cleanup ❓, corpus demands #2+#3 |
| [CIP-83](cips/0083-observability.md) | logs/stats/exit-causes as journald projection; `logNamespace:` | `cix logs`/`stats`/`system df` sugar ❓, per-app retention, logging drivers (❌ stands) |
| [CIP-84](cips/0084-closed-root.md) | mandatory closed root, whole-store ro | the ProtectSystem leak, NixOS two-paths lean, userns honesty follow-up |

Implementation status lives in .dev/LOG.md (75/76/78/80 shipped;
82 leg 1 and 83 in flight; 79/84 queued).

## Recorded-elsewhere, ledger row should cite it

- `LABEL`/display metadata → D54 (migrate.md row now cites it — done).
- Migrate-on-upgrade hooks → D48(f). Fixed uid/gid (`USER` row) → D48(d).
- `ARG`/build args ❓ → propose re-marking 🔁 citing D32 (generated
  Cixfile text is the parameter channel) + D46 (parametric composes);
  gitea's version-stamp case gets a documented pattern, not a mechanism.
  **Awaiting blessing.**

## Proposed one-line dispositions (awaiting Mathijs, batch-blessable)

- **`docker cp`** ❓ → ❌ + docs: role dirs are host paths (CIP-82 makes
  them self-describing); `cix inspect` names them.
- **`--name` stable handle** ❓ → compose provides stable names; a
  `cix run --name` is CIP-77-mechanical sugar, ⏳ until asked.
- **`STOPSIGNAL` / stop timeouts** ⏳ → mechanical manifest fields
  (`KillSignal=`, `TimeoutStopSec=`); small track when a corpus app
  needs it, no design required.
- **Namespace sharing modes (`--ipc/--pid/--uts`)** ⏳❓ → D43 pods are
  the sharing unit (`JoinsNamespaceOf=` is the mechanism); standalone
  sharing flags stay ❌.
- **Restart policy knobs** ⏳ → LIVENESS is the restart opt-in (CIP-79);
  `Restart=`/`RestartSec` tuning fields are compose-mechanical, ride the
  health implementation.
- **`docker init` generator** ❓ → the migrate prompt is the current
  generator; a `cix init` skeleton is tooling-era ⏳.
- **Docker Offload** ❓ → ❌; nix remote builders are the delegation
  story.
- **AppArmor/SELinux labeling** ❓ → host policy, out of manifest scope;
  revisit on a real SELinux-host user.
- **Desktop Enhanced Container Isolation** ❓ → ❌; different threat
  model, out of thesis.
- **Authorization plugins / policy point** ❓ → deferred to the
  server/reconciler era (D9); no plugin interface ever.
- **Engine API / SDKs** ❓ → reconciler era (D9).
- **Remote contexts / `--host`** ❓ → ssh is the transport; sugar ⏳.
- **`docker mcp`** ❓ → ❌ irrelevant to the runtime.
- **Capabilities coverage beyond NET_BIND_SERVICE** ❓ → grow
  claim-by-claim with dogfood (CIP-78 minted gpu/device); never a raw
  `--cap-add`.

## Era-parked (decided deferrals, no action now)

- **Publish era** (D17/D35): push, login/auth, entry signing, mirror
  redistribution, pull-through fill, hub/orgs/search/webhooks, SBOM &
  attestation exchange formats (+ Scout-class analysis on closures).
- **Networking era** (D26/D27/D49): bridge/DNS, finer isolation,
  network objects/drivers, `--hostname/--dns/--ip`, per-netns sysctls,
  port remapping/NAT & bind-address policy, the filtered resolver copy
  (CIP-84 §5).
- **Compose v1+** (D30 ledger): replicas/scale, resource limits (slice
  properties), configs objects, reusable top-level objects, live
  `update`.

## Errors found during the sweep — fixed

All three fixed 2026-08-01: the duplicate "registry mirrors" row in
docker.md merged (the D35 row stands), migrate.md's `LABEL` row cites
D54, corpus.md §4.1 points at CIP-79.

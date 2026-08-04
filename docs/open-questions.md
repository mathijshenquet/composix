# Open questions inventory — migration docs & compat ledgers

Status: swept 2026-08-02 after the adopted CIP implementation wave. Sources:
every ❓/⏳ row in docs/docker.md, the gap rows in docs/migrate.md,
docs/corpus.md §4, and the deferral notes in docs/design.md. New
decisions land as CIPs (cips/); this doc tracks what remains
genuinely open.

## Resolved into adopted CIPs (2026-08-01)

| CIP | Decided | Absorbed |
| --- | --- | --- |
| [CIP-75](../cips/accepted/0075-timers.md) | compose `schedule:` on raw OnCalendar | corpus demand #4, cron/scheduler sidecars |
| [CIP-76](../cips/accepted/0076-devloop.md) | `cix watch`; sync ❌ forever | compose `watch` ❓, live-reload binds |
| [CIP-77](../cips/accepted/0077-run-unary-compose.md) | `cix run` = unary compose invariant | the recurring "what about cix run?" |
| [CIP-78](../cips/accepted/0078-devices.md) | `CLAIM` vocabulary; `CLAIM gpu`/`device`, `SHM` | `--device/--gpus` ⏳, `--shm-size` ⏳, `--group-add` ❓, tmpfs ❓, compose `--privileged` ❓ (stays ❌) |
| [CIP-79](../cips/accepted/0079-health.md) | READINESS/LIVENESS; health graph banned | `HEALTHCHECK` rows, corpus demand #1 |
| [CIP-80](../cips/accepted/0080-exec-naming.md) | EXEC→START, SETUP→START_PRE | the EXEC bikeshed |
| [CIP-81](../cips/accepted/0081-secrets.md) | SECRET/LoadCredential; fetch tokens + host-side consent | secrets rows, BuildKit secret mounts ❓, `docker pass` ❓, credential stores ❓, corpus demand #8 |
| [CIP-82](../cips/accepted/0082-dirs.md) | dirs: claims model, overlay backing, `DIR`, lifecycle table | `-v/--mount`, named volumes + volume CLI ❓, bind mounts ⏳, `volume prune`/mutable-GC ❓, `container prune` cleanup ❓, corpus demands #2+#3 |
| [CIP-83](../cips/accepted/0083-observability.md) | logs/stats/exit-causes as journald projection; `logNamespace:` | `cix logs`/`stats`/`system df` sugar ❓, per-app retention, logging drivers (❌ stands) |
| [CIP-84](../cips/accepted/0084-closed-root.md) | mandatory closed root, whole-store ro | the ProtectSystem leak, NixOS two-paths lean, userns honesty follow-up |
| [CIP-85](../cips/accepted/0085-compose-tree.md) | recursive compose tree and host-root grammar | nested composites, path identity, subtree locks/repins, mutable host roots |
| [CIP-86](../cips/accepted/0086-netns.md) | D49 pod/netns realization | pod namespaces, fd-first publish, persisted egress IPAM; D26/D27 stay separate |
| [CIP-87](../cips/accepted/0087-read-set-keying.md) | traced read-set step keys | early cutoff and cold divergence audit |
| [CIP-88](../cips/accepted/0088-builder-ergonomics.md) | builder ergonomics | stats, vendored dev environments, source metadata, junk lint |
| [CIP-89](../cips/accepted/0089-thinning-round.md) | owned-module thinning | module maps, 2000-LOC tripwire, alpha compatibility audit |
| [CIP-90](../cips/accepted/0090-test-hygiene.md) | boundary configuration and structural tour reads | process-global test state and screen-scraping failure classes |

Implementation status lives in .dev/LOG.md. CIPs 75–83 and 86–88 are built; CIP-84 phase 1,
CIP-85 leg 1 plus netns/publish, CIP-89 leg 1, and CIP-90 leg A are built. Every scoped leg on the
adopted board is CI-confirmed; later legs remain explicit in their CIPs.

## Recorded-elsewhere, ledger row should cite it

- `LABEL`/display metadata → D54 (migrate.md row now cites it — done).
- Migrate-on-upgrade hooks → D48(f). Fixed uid/gid (`USER` row) → D48(d).
- `ARG`/build args ❓ → propose re-marking 🔁 citing D32 (generated
  Cixfile text is the parameter channel) + D46 (parametric composes);
  gitea's version-stamp case gets a documented pattern, not a mechanism.
  **Awaiting blessing.**

## Resolved agent investigations

- **netns activation under load** (resolved 2026-08-04): 20 contended
  pre-fix `scenario-netns` runs reproduced one exact closed-root activation
  failure and two stale namespace paths. The generated dependency graph was
  already correct. The real race was interrupted teardown: the suite's
  one-second manager stop timeout killed `ip netns delete`, leaving
  `/run/netns/cix-netns-b-netns`; immediate reactivation then failed `ip netns
  add` with `File exists`, and the member failed by dependency. Generated
  netns oneshots now have their own bounded 10-second stop budget. The focused
  VM and 20/20 identically contended post-fix runs passed with no netns stop,
  stale-path, or activation failure.

## Open for agents

- **CIP-79 adapter liveness on systemd 257** — the cix-owned HTTP/TCP
  `ExecStartPost` parent exits successfully, but its forked resident pinger is
  not retained and a healthy service later hits `WatchdogSec`. Add a systemd
  version gate or fix the retention mechanism; see the
  [Mastodon receipt](../corpus/migrate/mastodon/receipt.md#synchronous-receipt).

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
- **Restart policy knobs** ⏳ → LIVENESS is the shipped restart opt-in (CIP-79)
  with a fixed bounded policy; configurable `Restart=`/`RestartSec` tuning remains
  compose-mechanical follow-up work.
- **`docker init` generator** ❓ → the migrate prompt is the current
  generator; a `cix init` skeleton is tooling-era ⏳.
- **`ENV NAME=value` (Docker form, no spaces)** (Mathijs, 2026-08-04
  corpus review) → currently a parse error: `=` must be its own
  whitespace-separated token. Proposed disposition: keep one canonical
  form, improve the diagnostic to suggest `ENV NAME = value` — migrator
  muscle memory deserves a good error, not a second grammar.
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
- **Named-network era** (D26/D27): named network objects/drivers,
  service DNS/aliases, `talks-to` isolation, cross-composite and multi-host
  realization, `--hostname/--dns/--ip`, and per-netns sysctls. D49's pod
  netns, fd/proxy publish, persisted IPAM, bridge/veth egress, and closed-root
  resolver projection are built in CIP-86.
- **Compose v1+** (D30 ledger): replicas/scale, resource limits (slice
  properties), configs objects, reusable top-level objects, live
  `update`.

## Errors found during the sweep — fixed

All three fixed 2026-08-01: the duplicate "registry mirrors" row in
docker.md merged (the D35 row stands), migrate.md's `LABEL` row cites
D54, corpus.md §4.1 points at CIP-79.

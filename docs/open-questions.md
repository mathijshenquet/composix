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

- **CIP-79 adapter liveness on systemd 257** (re-evaluated 2026-08-04) —
  the focused health VM now runs the actual pinned systemd 257.6 PID 1 and
  keeps a healthy HTTP adapter alive for seven seconds after `cix up`, beyond
  its three-second watchdog window; the ordinary CI/flake VM is currently
  systemd 261. The reported Mastodon failure is therefore not reproduced in
  the available 257 universe. No version gate or weaker liveness behavior is
  justified without a reproducer carrying the original manager/package and
  generated-unit evidence.

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

- **Duplicate COPY against an already populated warm builder root**
  (regen wave 2, luna's Watchtower): Cix rejects a direct duplicate file
  destination when the builder's warm root is already populated. With
  `COPY context/ .` before `FETCH`, repeating `COPY context/go.mod go.mod`
  afterward fails on the next warm build with `BUILDER block destination
  "go.mod" is already populated`; the same applies to the other direct
  duplicate file copies. This is a warm-workspace product constraint, not a
  translation requirement; reproduce from the Watchtower regeneration and
  decide whether identical rewrites should be accepted or diagnosed earlier.
- **Bare `Error: Not a directory` from cix build** (track/fhsspike,
  2026-08-04): the patched directus build died with a context-free error
  after the full pnpm build stream — no path, no step attribution. A
  diagnosability defect (D73 spirit) and the actual remaining directus
  blocker; reproduce from the fhsspike branch and give the error its
  path + step context.
- **Degradation fallback drops the whole property set** (tour CI red,
  2026-08-04): a user manager that rejects one directive
  (`PrivatePIDs=` on CI's older systemd) makes cix retry without
  PrivateUsers, ProtectSystem, ProtectHome, PrivateTmp, AND BindPaths
  together — so one unknown directive costs the mount projection that
  chapter-1's nginx needed, and run output diverges per host. Degrade
  granularly: drop only what the manager rejected, keep the rest;
  D13's degraded path stays the floor, not the first fallback.
- **cix probe/audit tooling litters /tmp** (2026-08-04 incident): four
  `cix-fetch-probe-*` dirs (~1.1G each) and five `cix-build-cold-*`
  dirs (377M each) from past sessions survived on the tmpfs and
  exhausted its inode cap (node-tree-shaped contents), wedging every
  tool on the host. Probes and cold audits must clean their temp dirs
  on exit AND should relocate to a disk-backed product dir
  (~/.cache/cix/tmp) rather than the tmpfs — the node's tmpfs inode cap
  is admin-managed and not ours to raise (only home-manager runs here).
  ~/CLEANUP.md carries the sweep patterns as mitigation.
- **Lock-scale observation** (track/fhsspike, 2026-08-04): the directus
  builder run grew `Cixfile.lock` by ~148k lines of step observations /
  dev-env data. Possibly correct-but-heavy CIP-87/88 output on a huge
  read set; assess whether the lock format needs aggregation before
  big-ecosystem cases become routine.

## Resolved 2026-08-04

- **CONFIGDIR path freedom** (track/runfixes): runner validation now accepts
  every clean absolute path that the builder accepts. `ConfigurationDirectory=`
  mirrors the full path under its unit root and binds it back, including the
  in-namespace `CONFIGURATION_DIRECTORY` environment value; the focused VM
  proves `/config/probe` is writable.
- **Service `localhost`** (track/runfixes): sealed roots now provide a
  versioned minimal `/etc/hosts` with `127.0.0.1` and `::1` localhost entries.
  A declared item `/etc/hosts` mount deterministically overlays that skeleton;
  the focused VM proves both paths.

- **EXPECT not validated against the recorded pin on warm builds**
  (regen wave 1, luna's traefik, verified 2026-08-04): a Cixfile with a
  wrong/copy-pasted EXPECT builds green indefinitely while the fetch
  memo-hits — the mismatch only fires on a true refetch. Observed: both
  traefik fetches declare the same EXPECT, the lock records the same
  narHash for both pins while its stepMemos show different content, and
  corrupting one EXPECT surfaced a mismatch naming the *old* declared
  value. cix should cross-check declared EXPECT against the recorded
  pin at plan time (string compare) and error on divergence; reproduce
  from `track/regen1`'s traefik case and root-cause the identical-pin
  recording.
- **Unstable-API FETCH content is EXPECT-hostile** (same case): pinning
  GitHub's release-metadata JSON captures download counters that mutate
  every refetch, so the pin fails on any cache loss. Teaching nuance
  for migrate.md (normalize volatile JSON to the needed fields inside
  the fetch, or fetch the asset URL directly) + candidate lint.

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

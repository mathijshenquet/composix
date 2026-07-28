# The Docker ledger

Every Docker concept gets a *conscious* disposition — adopted, adapted, rejected, or deferred —
nothing by accident. Coarse-grained first; areas where composix already exists (index, run) are
tracked finer. Every ❓ goes through Mathijs before it becomes a decision. Dispositions cite
DESIGN.md decisions where they exist.

Legend: ✅ have · 🔁 adapted (solved differently) · ❌ rejected (with why) · ⏳ deferred (with
target era) · ❓ needs discussion.

## 1. Images, naming & distribution (fine — part 1 built)

| docker | disposition |
| --- | --- |
| image (artifact) | ✅ spec'd store item (vocabulary naming still open) |
| tag (mutable pointer) | ✅ `cix tag` (D5, D7: tags are GC roots) |
| digest (`@sha256:…`) | ✅ the store path *is* the digest (D12); no `@` syntax needed |
| registry + pull | ✅ `cix serve` / `cix pull` (D6, D17) |
| push | ⏳ deliberate (D17): later = "ask a server to publish for you", ssh transport first |
| default registry (docker.io) | ❌ by design: bare names are local, ever (D12) |
| official images (`library/`) | 🔁 cixpkgs (planned; examples/ is the seed) |
| multi-arch manifest lists | ✅ per-system outputs (D14) |
| `docker login` / registry auth | ⏳ arrives with push; authorization is server-side (D17) |
| content trust / signing | ✅ nix path signatures + `trustedKeys` in entries |
| `save` / `load` (tar transport) | 🔁 `nix copy --to file://…/ssh://…` is native and better |
| `image inspect` | 🔁 informative URL page (D18) + `cix ls -l`; a `cix inspect` ❓ |
| layers / `image history` | ❌ no layers; provenance = `drvPath` + `nix log` |
| build-cache push/pull | 🔁 binary caches are exactly this, natively |
| `image prune` / dangling | 🔁 `cix untag` + nix GC; `cix prune` sugar ⏳ |
| registry mirrors | 🔁 `substituters` list in entries (D6) |
| registry HTTP API | ✅ one negotiated URL space (D18) — richer than docker's |

## 2. Running (fine — part 2 built)

| docker | disposition |
| --- | --- |
| `run` | ✅ `cix run` (transient hardened unit) |
| `-e` env | ✅ typed env schema + `-e` (validated) |
| `-p` port publish | 🔁 declared ports = the network grant; host networking, no NAT. Remapping ❓ compose era |
| `-v` volumes | 🔁 role dirs (state/cache/logs/config/run, D11 narrowed); operator host-binds ⏳ compose |
| `--restart` policies | ⏳ compose (systemd `Restart=` natively) |
| `HEALTHCHECK` / health status | ⏳ parsed today, wired in compose era |
| `logs` | 🔁 journald; `cix run` streams, `journalctl -u cix-*` always works |
| `exec` (shell into container) | ❓ no container, but the unit's namespace exists — `cix exec` via nsenter? discuss |
| `attach` | 🔁 journal streaming |
| `stop` / `kill` / signals | ✅ systemctl stop; custom stop signal/timeouts ⏳ (spec v3 candidate) |
| `rm` / `--rm` | 🔁 transient units self-collect |
| `ps` | ✅ `cix ps` |
| `stats` / `top` | 🔁 cgroup accounting is native (`systemd-cgtop`); `cix stats` sugar ⏳ |
| `update` (live resource limits) | ⏳ compose era (limits are operator config) |
| `cp` | ❓ role dirs are plain host paths — document paths instead of a command? |
| `commit` (container → image) | ❌ fundamental: artifacts come from builds, never snapshots (purity) |
| `pause` / `unpause` | 🔁 `systemctl freeze/thaw` exists; sugar probably ❌ |
| `--user` (pick uid) | ❌ DynamicUser is the model; fixed uids are the anti-pattern we left |
| `--privileged` | ❌ against the cap-spec thesis (D20a); operator overrides live in compose ❓ |
| `--init` (tini) | ❌ systemd *is* the init |
| interactive `-it` containers | 🔁 out of scope for services; `nix run`/`nix shell` already cover one-offs |
| `create`/`start` (pre-created stopped containers) | ❌ transient units are run-or-nothing; persistent units arrive with compose |
| `restart` | 🔁 `systemctl restart`; policy ⏳ compose |
| `checkpoint` (CRIU) | ❌ niche, no systemd first-class support |
| resource flags (`--cpu-*` `--memory-*` `--blkio-*` `--ulimit` `--oom-*`) | ⏳ compose: slice/unit limits, systemd-native |
| namespace modes (`--ipc` `--pid` `--uts`) | 🔁 systemd sandboxing covers isolation; *sharing* modes ⏳ compose ❓ |
| `--device` / `--gpus` / `--device-cgroup-rule` | ⏳ deliberate (spec v2 deferral): needs a dogfood case; `DeviceAllow=` exists |
| `--read-only` | ✅ stronger as default: `ProtectSystem=strict` always |
| `--shm-size` | ⏳ postgres already brushed `/dev/shm`; no direct systemd knob, needs design |
| `--sysctl` | ❌ host policy; per-netns sysctls ⏳ networking era |
| `--name` | 🔁 unit names are systematic (`cix-run-<svc>-<nonce>`, compose: `cix-<comp>-<svc>`) |
| `--hostname` `--dns*` `--add-host` `--ip*` `--mac-address` `--network-alias` | ⏳ networking era, wholesale |
| `--group-add` (supplementary groups) | ❓ adjacent to device access; no case yet |
| docker-machinery flags (`--cidfile` `--detach-keys` `--label*` `--annotation` `--cgroup-parent` `--isolation` `--runtime` `--publish-all` `--volumes-from`) | ❌ no composix analog needed |

## 3. Building (part 4 designed — coarse)

| docker | disposition |
| --- | --- |
| Dockerfile | 🔁 Cixfile (D4) + always the `.nix` escape hatch |
| `FROM` / base images | 🔁 nixpkgs + ecosystem builders; no layer inheritance |
| `RUN` | ❌ imperative impure steps (part 4 position); blessed builders instead |
| `COPY` / `ADD` / `.dockerignore` | 🔁 source filtering |
| `ENV EXPOSE VOLUME ENTRYPOINT CMD WORKDIR HEALTHCHECK USER LABEL` | 🔁 SERVICE blocks → `cix-spec.json` |
| `ARG` / build args | ❓ |
| multi-stage builds | 🔁 derivations compose naturally |
| buildkit secret/ssh mounts | ❓ private deps — nix has netrc/access-tokens; needs a story |
| reproducible builds | ✅ the whole point |
| Dockerfile here-documents | ✅ the Cixfile design *is* heredoc-first (`FILE`/`SCRIPT <<EOF`) |
| `STOPSIGNAL` | ⏳ spec v3 candidate (`KillSignal=`), with stop timeouts |
| `SHELL` `ONBUILD` `MAINTAINER` parser directives | ❌ SCRIPT has a fixed shell; no image inheritance; no parser magic |
| `RUN --mount=cache/bind/tmpfs` `--network` `--security` | ❌ falls with RUN itself; nix builders answer these |
| buildx / bake / builder management | ❌ nix is the builder; remote/multi-platform 🔁 nix distributed builds |

## 4. Storage (coarse)

| docker | disposition |
| --- | --- |
| named volumes (shared between services) | ⏳ compose era |
| bind mounts (host paths in) | ⏳ compose era, operator territory |
| tmpfs mounts | ✅ PrivateTmp; more ⏳ |
| volume drivers / plugins | ❌ filesystems are the host's business |
| `volume prune`/`update` | 🔁 role dirs are plain host paths; lifecycle via `systemctl clean` + GC |

## 5. Networking (coarse — the biggest *conscious* gap)

| docker | disposition |
| --- | --- |
| port publish / NAT | 🔁 host networking + explicit port allocation (MVP position, part 3) |
| bridge networks, service DNS | ⏳ compose-era decision (per-slice netns? socket activation?) |
| network isolation | ✅ coarse today (no ports ⇒ PrivateNetwork); finer ⏳ |
| overlay / multi-host | ❌ now; k8s-lite ambitions much later |
| `--link` (legacy) | ❌ |

## 6. Compose (part 3 designed — coarse until built)

| docker | disposition |
| --- | --- |
| compose.yml services | 🔁 part 3; surface language TBD (prototyping planned) |
| `depends_on` / ordering | ⏳ systemd After/Wants natively |
| scale / replicas | ⏳ template units (`@n`) |
| env_file / secrets | ⏳ `LoadCredential=` (D20b: operator territory) |
| resource limits | ⏳ slice properties, natively |
| project namespacing | ✅ designed: `cix-<composite>.slice`/`.target` |
| `up` / `down` / rollback | ✅ designed: resolve→lock→build→activate, per-composite profiles (D9) |
| `watch` (dev mode) | ❓ interesting dev loop, unscoped |
| profiles | ❓ |
| `configs` top-level element | ⏳ compose config story (`ConfigurationDirectory` content) |
| `version` marker (obsolete) | ❌ |

## 7. Daemon & platform

| docker | disposition |
| --- | --- |
| dockerd (the daemon) | ❌ systemd is the runtime; cix is a CLI + later a small reconciler (D9) |
| `docker context` / remote hosts | ❓ ssh is the transport today; `cix --host` sugar maybe ⏳ |
| events API | 🔁 journald/systemd events |
| logging drivers | ❌ journald; forwarding is journald's job |
| storage drivers | ❌ the nix store |
| plugins | ❌ |
| rootless mode | 🔁 `--user` degraded dev mode exists (D13); full rootless not a goal |
| Docker Desktop / GUI | ❌ |

## 8. Orchestration (swarm)

❌ wholesale. The compose reconciler (D9) is the k8s-lite seed; anything beyond single-host
comes much later, consciously.

## 9. Security

| docker | disposition |
| --- | --- |
| seccomp profiles | ✅ `SystemCallFilter=@system-service` default; custom profiles ❌ (D20a) |
| capabilities (`--cap-add`) | ✅ semantic grants only (e.g. port<1024 ⇒ NET_BIND_SERVICE, spec v2) |
| userns-remap | ✅ DynamicUser + idmapped mounts, native and better |
| apparmor/selinux | ❓ host policy, likely out of spec scope |
| secrets | ⏳ `LoadCredential=`, compose era |
| SBOM / scanning (scout) | 🔁 the closure *is* an exact inventory, free; tooling ⏳ |
| provenance/attestations | 🔁 drvPath + path signatures; richer story ⏳ |

## 10. Hub & ecosystem

| docker | disposition |
| --- | --- |
| hub search/explore | ⏳ the serve pages (D18) are the seed of "explore" |
| `docker dhi` (hardened images) / `docker model` (AI artifacts) | ❌ product catalog features, not runtime concepts |
| automated builds | ❌ CI's job |
| Misc CLI: `diff` `export` `import` `rename` | ❌ artifacts are immutable store items |
| `wait` `port` `version` `info` | 🔁 trivial equivalents where useful |

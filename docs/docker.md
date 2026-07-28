# The Docker ledger

Every Docker concept gets a *conscious* disposition—adopted, adapted, rejected, or deferred—
nothing by accident. This is a ledger against Docker as it exists, not against a convenient
subset of it. “Designed” and “provided by Nix or systemd” do not mean “shipped by composix.”
Every ❓ goes through Mathijs before it becomes a decision. Dispositions cite `design.md`
decisions where they exist.

Legend: ✅ have · 🔁 adapted (solved differently) · ❌ rejected (with the loss named) ·
⏳ deferred (with a target era) · ❓ needs discussion.

## The honest gaps

This is the case against composix today:

- **It cannot run an existing image.** There is no OCI image, Docker image, registry-v2, or
  containerd interoperability layer. A team cannot point `cix run` at any of the
  [millions of images in Docker Hub](https://docs.docker.com/docker-hub/); it must first create
  a Nix store item and a composix spec. That is a new packaging ecosystem, not a drop-in runtime.
- **It is Linux + systemd only.** The product target is the root-managed system systemd. The
  degraded user-manager mode is not a portable runtime. There is no equivalent to
  [Docker Desktop on macOS](https://docs.docker.com/desktop/setup/install/mac-install/) or
  [Windows](https://docs.docker.com/desktop/setup/install/windows-install/), no managed Linux VM,
  no file-sharing layer, no Windows-container story, and no GUI.
- **The ecosystem comparison is not close.** Docker Hub advertises millions of images plus
  official, verified-publisher, hardened, and sponsored catalogs; `cixpkgs` is only a plan and
  this repository has two hand-built examples. Docker also documents a working
  [pull-through cache](https://docs.docker.com/docker-hub/image-library/mirror/); an entry's
  substituter list selects caches but does not discover, populate, govern, or garbage-collect a
  mirror.
- **Security defaults are young and locally asserted.** composix has a promising systemd
  sandbox, but no published threat model, compatibility corpus, external audit, CVE response
  history, policy profiles, or evidence that its default syscall and mount rules cover the
  diversity Docker's [security model](https://docs.docker.com/engine/security/) has accumulated.
  “Stricter” is not established by listing unit directives.
- **Rootless is incomplete by design.** `cix run --user` drops `DynamicUser`, bind mounts, and
  parts of the sandbox and is explicitly degraded. Docker's
  [rootless mode](https://docs.docker.com/engine/security/rootless/) runs both daemon and
  containers without root while retaining an image/container lifecycle and networking.
- **Networking is not a container network.** There is no per-app network namespace for networked
  services, bridge/NAT, service DNS, aliases, address management, published-port inventory,
  overlay, or isolation between two apps that both need the network. `PrivateNetwork=yes` only
  handles the no-network case.
- **The operational surface is seven commands.** There is no composix-native logs query, inspect,
  events, stats, top, exec/debug, health status, wait, resource update, disk-usage report, network
  inspection, or volume inspection. There are no demonstrated monitoring exporters, CI actions,
  IDE integrations, SDKs, or [Testcontainers](https://docs.docker.com/testcontainers/cloud/)
  compatibility.
- **A Compose migration is a cliff.** Compose is not implemented and its surface language is
  deliberately undecided. A Docker Compose shop must repackage every image, translate every
  health check, mount, network, secret, dependency, and operator override, then accept a
  single-host systemd-only deployment before it can test equivalence.
- **The build bridge is still prose.** Cixfile v1 is designed but there is no `cix build` command.
  It intentionally omits general `RUN`, so many Dockerfiles cannot be mechanically translated
  even after the command exists.
- **There is no compatibility or performance evidence.** The project has two dogfood services,
  not a representative application corpus. It publishes no cold-pull byte counts, shared-content
  deduplication comparison, startup benchmarks, failure-recovery tests, upgrade matrix, or
  long-running reliability data against Docker.

The Nix store and systemd are real strengths. They are not receipts for the missing product
surface above.

## 1. Images, naming & distribution (fine—part 1 built)

| docker | disposition |
| --- | --- |
| [image (artifact)](https://docs.docker.com/get-started/docker-concepts/the-basics/what-is-an-image/) | ✅ spec'd store item (vocabulary naming still open) |
| [tag (mutable pointer)](https://docs.docker.com/reference/cli/docker/image/tag/) | ✅ `cix tag` (D5, D7: tags are GC roots) |
| [digest (`@sha256:…`)](https://docs.docker.com/dhi/core-concepts/digests/) | ✅ the store path *is* the digest (D12); no `@` syntax needed. ❓ A store path includes a Nix name and hashes a Nix closure identity, not an OCI manifest; document precisely which digest properties are equivalent. |
| [registry + pull](https://docs.docker.com/reference/cli/docker/image/pull/) | ✅ `cix serve` / `cix pull` (D6, D17) |
| [push](https://docs.docker.com/reference/cli/docker/image/push/) | ⏳ deliberate (D17): later = “ask a server to publish for you,” ssh transport first |
| [default registry (`docker.io`)](https://docs.docker.com/docker-hub/) | ❌ by design: bare names are always local (D12). Given up: zero-configuration access to a shared public namespace. |
| [official images (`library/`)](https://docs.docker.com/docker-hub/image-library/trusted-content/) | 🔁 cixpkgs (planned; `examples/` is the seed). ❓ Two examples do not yet earn an ecosystem adaptation claim. |
| [multi-platform images / manifest lists](https://docs.docker.com/build/building/multi-platform/) | ✅ per-system outputs (D14). ❓ This has metadata coverage, but no published evidence of cross-building, serving, and pulling every claimed system. |
| [`docker login` / registry auth](https://docs.docker.com/reference/cli/docker/login/) | ⏳ arrives with push; authorization is server-side (D17) |
| [content trust / signing](https://docs.docker.com/engine/security/trust/) | ✅ Nix path signatures + `trustedKeys` in entries. ❓ Specify key rotation, revocation, delegation, policy enforcement, and unsigned-path behavior before claiming operational parity. |
| [`save` / `load` (tar transport)](https://docs.docker.com/reference/cli/docker/image/save/) | 🔁 `nix copy --to file://…/ssh://…` is native. ❓ “Better” is unevidenced: Docker emits a portable stream with tags and layers; document the exact offline round trip and metadata retained by the Nix replacement. |
| [`image ls` / `image rm`](https://docs.docker.com/reference/cli/docker/image/ls/) | 🔁 `cix ls` / `cix untag` plus Nix GC; removal of shared store content is deliberately indirect |
| [`image inspect`](https://docs.docker.com/reference/cli/docker/image/inspect/) | 🔁 informative URL page (D18) + `cix ls -l`; a `cix inspect` ❓. The current views expose much less runtime/config metadata than Docker inspect. |
| [layers / `image history`](https://docs.docker.com/reference/cli/docker/image/history/) | ❌ no layers; provenance = `drvPath` + `nix log`. Given up: layer-level transfer reuse, authoring history, and a ubiquitous debugging vocabulary. |
| [build-cache export/import](https://docs.docker.com/build/cache/backends/) | 🔁 Nix binary caches provide remote build results. ❓ Compare cache keys, partial reuse, multi-stage behavior, export modes, and cold-transfer bytes before calling them “exactly” the same. |
| [`image prune` / dangling images](https://docs.docker.com/reference/cli/docker/image/prune/) | 🔁 `cix untag` + Nix GC; `cix prune` sugar ⏳ |
| [registry mirrors / pull-through cache](https://docs.docker.com/docker-hub/image-library/mirror/) | 🔁 `substituters` list in entries (D6). ❓ A list of content sources is not itself a pull-through mirror with upstream fill, freshness checks, and cache lifecycle. |
| [registry HTTP API](https://docs.docker.com/reference/api/registry/latest/) | ✅ one negotiated URL space (D18). ❓ “Richer” does not offset incompatibility with registry clients, OCI tooling, or registry middleware. |
| [`image import`](https://docs.docker.com/reference/cli/docker/image/import/) | ❌ artifacts are built, never imported from mutable filesystem snapshots. Given up: a direct migration path from rootfs tarballs. |
| [`docker manifest`](https://docs.docker.com/reference/cli/docker/manifest/) | ❓ D14 stores per-system outputs, but there is no client surface to inspect, annotate, create, or push a multi-platform manifest. |

**Residuals.** Docker can pull and run existing OCI/Docker images, stream tagged images through
`save`/`load`, push with established authentication, speak a standard registry API, and reuse
layers across unrelated images. composix can only distribute already-built Nix closures described
by its own entries. D5–D7, D10, D14, D17, and D18 plan a coherent Nix-native path, but none plans
OCI interoperability; D17 explicitly leaves push and registry authorization for later. No
measured cold-transfer or shared-content result establishes when a Nix closure is smaller or
larger than Docker's compressed layers.

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

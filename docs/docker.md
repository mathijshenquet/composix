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
| [`run`](https://docs.docker.com/reference/cli/docker/container/run/) | ✅ `cix run` (transient hardened unit) |
| [`-e` environment](https://docs.docker.com/reference/cli/docker/container/run/#env) | ✅ declared string environment + validated `-e` (D21) |
| [`-p` port publish](https://docs.docker.com/reference/cli/docker/container/run/#publish) | 🔁 declared ports = the network grant; host networking, no NAT. Remapping ❓ compose era. ❓ Host exposure without NAT, bind-address choice, collision management, or a port inventory is a materially smaller facility. |
| [`-v` / `--mount`](https://docs.docker.com/engine/storage/bind-mounts/) | 🔁 role dirs (state/cache/logs/config/run, D11 narrowed); operator host-binds ⏳ compose. ❓ Managed FHS role directories do not replace arbitrary read-only/read-write binds, propagation, consistency, or subpaths. |
| [`--restart` policies](https://docs.docker.com/engine/containers/start-containers-automatically/) | ⏳ compose (systemd `Restart=` natively) |
| [`HEALTHCHECK` / health status](https://docs.docker.com/reference/dockerfile/#healthcheck) | ⏳ parsed today, wired in compose era |
| [`logs`](https://docs.docker.com/reference/cli/docker/container/logs/) | 🔁 journald; foreground `cix run` streams and `journalctl -u cix-*` works. ❓ There is no `cix logs`, stable selector, per-app retention contract, or logging-driver integration. |
| [`exec` (command in container)](https://docs.docker.com/reference/cli/docker/container/exec/) | ❓ no container, but the unit's namespaces exist—would `cix exec` enter them, and under which identity/capabilities? |
| [`attach`](https://docs.docker.com/reference/cli/docker/container/attach/) | 🔁 journal streaming. ❓ Journal following is output-only and is not attach parity for stdin, signals, detach keys, or the process TTY. |
| [`stop` / `kill` / signals](https://docs.docker.com/reference/cli/docker/container/stop/) | ✅ `systemctl stop`; custom stop signal/timeouts ⏳ (spec v3 candidate). ❓ Calling a generic supervisor escape hatch “have” leaves no composix CLI, object selector, or `kill` equivalent. |
| [`rm` / `--rm`](https://docs.docker.com/reference/cli/docker/container/rm/) | 🔁 transient units self-collect |
| [`ps`](https://docs.docker.com/reference/cli/docker/container/ls/) | ✅ `cix ps` |
| [`stats` / `top`](https://docs.docker.com/reference/cli/docker/container/stats/) | 🔁 cgroup accounting is available through `systemd-cgtop`; `cix stats` sugar ⏳. ❓ Docker supplies per-container CLI output and an API; the underlying accounting primitive is not the product surface. |
| [`update` (live resource limits)](https://docs.docker.com/reference/cli/docker/container/update/) | ⏳ compose era (limits are operator config) |
| [`cp`](https://docs.docker.com/reference/cli/docker/container/cp/) | ❓ role dirs are plain host paths—would path documentation replace copying to/from the immutable item and private namespaces? |
| [`commit` (container → image)](https://docs.docker.com/reference/cli/docker/container/commit/) | ❌ fundamental: artifacts come from builds, never snapshots (purity). Given up: capturing a debugged or manually modified runtime as a distributable artifact. |
| [`pause` / `unpause`](https://docs.docker.com/reference/cli/docker/container/pause/) | 🔁 `systemctl freeze/thaw` exists; sugar probably ❌. ❓ Confirm semantics and cgroup-version support before presenting the commands as interchangeable. |
| [`--user` (pick uid)](https://docs.docker.com/reference/cli/docker/container/run/#user) | ❌ `DynamicUser` is the model; fixed UIDs are refused. Given up: compatibility with images, mounted files, licenses, and protocols that require a known numeric identity. |
| [`--privileged`](https://docs.docker.com/reference/cli/docker/container/run/#privileged) | ❌ against the capability-spec thesis (D20a); operator overrides live in compose ❓. Given up: workloads that need broad device/kernel access and the common diagnostic escape hatch. |
| [`--init`](https://docs.docker.com/reference/cli/docker/container/run/#init) | ❌ systemd is the service manager. Given up: Docker's portable in-container child reaping/forwarding behavior when the same image runs under another runtime. |
| [interactive `-it` containers](https://docs.docker.com/reference/cli/docker/container/run/#foreground) | 🔁 out of scope for services; `nix run`/`nix shell` cover Nix-native one-offs. ❓ They cannot open an existing image or reproduce its filesystem, environment, entrypoint, and user. |
| [`create` / `start` (stopped containers)](https://docs.docker.com/reference/cli/docker/container/create/) | ❌ transient units are run-or-nothing; persistent units arrive with compose. Given up: prepare/inspect/start workflows and a durable stopped-object inventory. |
| [`restart`](https://docs.docker.com/reference/cli/docker/container/restart/) | 🔁 `systemctl restart`; policy ⏳ compose. ❓ Again, this is an operator escape hatch rather than a `cix` lifecycle command. |
| [`checkpoint` (CRIU)](https://docs.docker.com/reference/cli/docker/checkpoint/) | ❌ niche, no systemd first-class support. Given up: checkpoint/restore, live migration building blocks, and stateful fast restart. |
| [resource flags (`--cpu-*`, `--memory-*`, `--blkio-*`, `--ulimit`, `--oom-*`)](https://docs.docker.com/reference/cli/docker/container/run/#runtime-constraints-on-resources) | ⏳ compose: slice/unit limits, systemd-native |
| [namespace modes (`--ipc`, `--pid`, `--uts`)](https://docs.docker.com/reference/cli/docker/container/run/#ipc) | 🔁 systemd sandboxing covers some isolation; *sharing* modes ⏳ compose ❓. “Covers” needs a field-by-field namespace audit, especially for networked units. |
| [`--device` / `--gpus` / `--device-cgroup-rule`](https://docs.docker.com/reference/cli/docker/container/run/#device) | ⏳ deliberate (spec v2 deferral): needs a dogfood case; `DeviceAllow=` exists |
| [`--read-only`](https://docs.docker.com/reference/cli/docker/container/run/#read-only) | ✅ `ProtectSystem=strict` by default. ❓ Verify the effective writable mount set and namespace behavior against Docker's read-only root filesystem rather than inferring parity from one directive. |
| [`--shm-size`](https://docs.docker.com/reference/cli/docker/container/run/#shm-size) | ⏳ PostgreSQL already brushed `/dev/shm`; no direct systemd knob, needs design |
| [`--sysctl`](https://docs.docker.com/reference/cli/docker/container/run/#sysctl) | ❌ host policy; per-netns sysctls ⏳ networking era. Given up today: safe namespaced tuning required by some databases, proxies, and network appliances. |
| [`--name`](https://docs.docker.com/reference/cli/docker/container/run/#name) | 🔁 unit names are systematic (`cix-run-<svc>-<nonce>`; compose plans `cix-<comp>-<svc>`). ❓ A random run nonce is not a user-chosen stable handle. |
| [`--hostname`, `--dns*`, `--add-host`, `--ip*`, `--mac-address`, `--network-alias`](https://docs.docker.com/reference/cli/docker/container/run/#network-settings) | ⏳ networking era, wholesale |
| [`--group-add` (supplementary groups)](https://docs.docker.com/reference/cli/docker/container/run/#additional-groups) | ❓ adjacent to device access; no case yet |
| [Docker-machinery flags (`--cidfile`, `--detach-keys`, `--label*`, `--annotation`, `--cgroup-parent`, `--isolation`, `--runtime`, `--publish-all`, `--volumes-from`)](https://docs.docker.com/reference/cli/docker/container/run/#options) | ❌ no composix analog planned. Given up: machine-readable identity handoff, selectable runtimes/isolation, metadata-based automation, cgroup placement, automatic publication, and volume sharing. |
| [`container inspect`](https://docs.docker.com/reference/cli/docker/container/inspect/) | ❓ `cix ps` only lists running units; define how users obtain resolved environment, mounts, sandbox, ports, state paths, status, and exit cause. |
| [`container prune`](https://docs.docker.com/reference/cli/docker/container/prune/) | ❓ transient units should collect automatically, but persistent compose units and role-directory lifecycle have no equivalent cleanup contract yet. |
| [`docker debug`](https://docs.docker.com/reference/cli/docker/debug/) | ❓ Docker now offers a toolbox shell even for slim images; composix has neither an image shell nor a packaged debug-tool injection story. |

**Residuals.** Docker supplies a durable container object, stdin/TTY attachment, mutable root
filesystem, explicit namespace and identity controls, NAT/port publication, arbitrary mounts,
devices, resource knobs, health state, debug/exec, and a remotely queryable API. composix ships a
good transient-service path and `ps`; most other comparisons invoke raw systemd tools or future
compose work. D11, D20a, and D22 deliberately narrow mounts and privileges, but they do not cover
the compatibility lost. D9 plans persistent compose units; it does not yet specify their complete
lifecycle or inspection surface.

## 3. Building (part 4 designed — coarse)

| docker | disposition |
| --- | --- |
| [Dockerfile](https://docs.docker.com/reference/dockerfile/) | 🔁 Cixfile (D4) + always the `.nix` escape hatch. ❓ Cixfile is unimplemented, omits general build steps, and cannot yet substantiate an adaptation claim. |
| [`FROM` / base images](https://docs.docker.com/reference/dockerfile/#from) | 🔁 nixpkgs + ecosystem builders; no layer inheritance. ❓ This only helps software packaged in Nix or newly packaged for it; it cannot consume an arbitrary base image. |
| [`RUN`](https://docs.docker.com/reference/dockerfile/#run) | ❌ imperative impure steps; blessed builders instead. Given up: the universal escape hatch that makes existing installation instructions and most Dockerfiles directly expressible. |
| [`COPY` / `ADD` / `.dockerignore`](https://docs.docker.com/build/concepts/context/#dockerignore-files) | 🔁 Cixfile source assembly/filtering. ❓ D4 describes `COPY` and inline files, but the command and filtering semantics are not built. |
| [`ENV`, `EXPOSE`, `VOLUME`, `ENTRYPOINT`, `CMD`, `WORKDIR`, `HEALTHCHECK`, `USER`, `LABEL`](https://docs.docker.com/reference/dockerfile/#overview) | 🔁 Cixfile `SERVICE` blocks → `cix-spec.json`. ❓ The proposed schema has no faithful equivalent for all of these (notably image labels, arbitrary user, working directory, and wired health). |
| [`ARG` / build args](https://docs.docker.com/reference/dockerfile/#arg) | ❓ decide how configurable builds coexist with pinned inputs and cache identity |
| [multi-stage builds](https://docs.docker.com/build/building/multi-stage/) | 🔁 derivations compose naturally. ❓ Show how Cixfile users—not `.nix` authors—express private intermediate tools and selective copying. |
| [BuildKit secret/SSH mounts](https://docs.docker.com/build/building/secrets/) | ❓ private dependencies—Nix has netrc/access tokens, but the non-leaking Cixfile and remote-builder story is unspecified |
| [reproducible builds](https://docs.docker.com/build/ci/github-actions/reproducible-builds/) | ✅ the point of the Nix foundation. ❓ The product still needs locked-input enforcement and empirical rebuild checks; Nix permits impure and non-reproducible derivations. |
| [Dockerfile here-documents](https://docs.docker.com/reference/dockerfile/#here-documents) | ✅ the Cixfile design is heredoc-first (`FILE`/`SCRIPT <<EOF`). ❓ “Have” currently means a design document, not executable syntax. |
| [`STOPSIGNAL`](https://docs.docker.com/reference/dockerfile/#stopsignal) | ⏳ spec v3 candidate (`KillSignal=`), with stop timeouts |
| [`SHELL`, `ONBUILD`, `MAINTAINER`, parser directives](https://docs.docker.com/reference/dockerfile/#parser-directives) | ❌ `SCRIPT` has a fixed shell; no image inheritance; no parser magic. Given up: inherited downstream triggers, per-image shell choice, frontend versioning, and direct compatibility with those Dockerfiles. |
| [`RUN --mount=cache/bind/tmpfs`, `--network`, `--security`](https://docs.docker.com/reference/dockerfile/#run) | ❌ falls with `RUN`; Nix builders are the proposed answer. Given up: concise per-step cache, secret-adjacent, network, and security controls familiar to BuildKit users. |
| [Buildx / Bake / builder management](https://docs.docker.com/build/buildx/) | ❌ Nix is the builder; remote/multi-platform 🔁 Nix distributed builds. Given up: the Docker CLI's named builders, Bake graph, standard exporters, driver choices, and existing CI actions. |
| [`docker init`](https://docs.docker.com/reference/cli/docker/init/) | ❓ no generator exists for a Cixfile, spec, Nix wrapper, or compose migration skeleton |
| [build attestations](https://docs.docker.com/build/metadata/attestations/) | ❓ `drvPath` is provenance metadata, but there is no standard SBOM/provenance attestation emission or policy story |

**Residuals.** Docker has an implemented, versioned Dockerfile frontend, arbitrary build steps,
multi-stage builds, secret/SSH mounts, cache exporters, attestations, named local/remote/cloud
builders, Bake, and mature CI actions. Cixfile currently has no parser or `cix build`; D4 and the
part-4 design only describe a narrower assembly language. The `.nix` escape hatch is powerful for
Nix authors but is a migration burden, not Dockerfile compatibility. Rejecting `RUN` consciously
trades broad packageability for discipline.

## 4. Storage (coarse)

| docker | disposition |
| --- | --- |
| [named volumes (including service sharing)](https://docs.docker.com/engine/storage/volumes/) | ⏳ compose era |
| [bind mounts (host paths in)](https://docs.docker.com/engine/storage/bind-mounts/) | ⏳ compose era, operator territory |
| [tmpfs mounts](https://docs.docker.com/engine/storage/tmpfs/) | ✅ `PrivateTmp`; more ⏳. ❓ A private `/tmp` and `/var/tmp` does not cover arbitrary tmpfs destinations, sizing, modes, or swap behavior. |
| [volume drivers / plugins](https://docs.docker.com/engine/extend/plugins_volume/) | ❌ filesystems are the host's business. Given up: portable compose declarations for remote, encrypted, cloud, clustered, and vendor storage backends. |
| [`volume prune` / `volume update`](https://docs.docker.com/reference/cli/docker/volume/) | 🔁 role dirs are plain host paths; lifecycle via `systemctl clean` + GC. ❓ Nix GC does not clean mutable role data, and no `cix` command inventories ownership or previews deletion. |
| [`volume create` / `inspect` / `ls` / `rm`](https://docs.docker.com/reference/cli/docker/volume/) | ❓ compose has not defined the named-volume object, metadata, sharing, lifecycle, or CLI surface |

**Residuals.** Docker provides named volume objects with create/list/inspect/remove/prune,
cross-service attachment, copy-up and `volume-subpath` behavior, and a driver interface. composix
today creates service-specific FHS role directories indirectly through systemd. D11 deliberately
covers app-path persistence, not shared volumes, arbitrary mounts, backup/restore, remote storage,
quota, snapshot, encryption, or a safe mutable-data garbage collector; compose merely defers those
questions.

## 5. Networking (coarse — the biggest *conscious* gap)

| docker | disposition |
| --- | --- |
| [port publish / NAT](https://docs.docker.com/engine/network/port-publishing/) | 🔁 host networking + explicit port allocation (MVP position, part 3). ❓ This gives networked services the host stack and is neither address translation nor per-service exposure control. |
| [bridge networks and service DNS](https://docs.docker.com/engine/network/drivers/bridge/) | ⏳ compose-era decision (per-slice netns? socket activation?) |
| [network isolation](https://docs.docker.com/engine/network/) | ✅ coarse today (no ports ⇒ `PrivateNetwork`); finer ⏳. ❓ A binary no-network switch does not isolate one networked composix app from another or from host interfaces. |
| [overlay / multi-host networking](https://docs.docker.com/engine/network/drivers/overlay/) | ❌ now; k8s-lite ambitions much later. Given up: encrypted multi-daemon service networks and the basis for multi-host orchestration. |
| [`--link` (legacy)](https://docs.docker.com/engine/network/links/) | ❌ legacy environment/hosts coupling is refused. Given up is minor because Docker itself recommends user-defined networks. |
| [`network create` / `connect` / `disconnect` / `inspect` / `ls` / `prune` / `rm`](https://docs.docker.com/reference/cli/docker/network/) | ❓ there is no composix network object or lifecycle surface |
| [host, none, macvlan, ipvlan network drivers](https://docs.docker.com/engine/network/drivers/) | ❓ the ledger only discusses bridge and overlay; decide whether these modes are unsupported, operator overrides, or future capabilities |

**Residuals.** Docker supplies per-container network namespaces, bridge/NAT, user-defined network
objects, embedded DNS, aliases, IPAM, several local drivers, overlay networks, inspection, and
connect/disconnect lifecycle. composix only denies networking entirely or places the service on
the host network. Part 3 explicitly chooses host networking for its MVP and merely opens the
per-slice-netns question, so neither D9 nor another current decision closes this gap.

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

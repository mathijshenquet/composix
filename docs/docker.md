# The Docker ledger

Every Docker concept gets a *conscious* disposition—adopted, adapted, rejected, or deferred—
nothing by accident. This is a ledger against Docker as it exists, not against a convenient
subset of it. “Designed” and “provided by Nix or systemd” do not mean “shipped by composix.”
Every ❓ goes through Mathijs before it becomes a decision. Dispositions cite `design.md`
decisions where they exist.

Legend: ✅ have · 🔁 adapted (solved differently) · ❌ rejected · ⏳ deferred (with a
target era) · ❓ needs discussion. The third column names what Docker still provides that
composix does not.

## Scope, stated once

Two facts are theses, not gaps — they will never change and everything below assumes them:
composix is **Linux + root-managed systemd only** (no
[Docker Desktop](https://docs.docker.com/desktop/) analog, `--user` is an explicitly degraded
dev mode), and it is **a new packaging ecosystem, not a drop-in runtime** — it will never *run*
an [OCI/Docker image](https://docs.docker.com/get-started/docker-concepts/the-basics/what-is-an-image/);
software enters as a nix store item. Being young is also not listed as a gap below: fewer
integrations, no audit history, and a two-example catalog are what "early" means, and only
time and adoption fix them.

## The gaps that matter

The actionable case against composix today — each item is either a decision to make or work to
schedule:

- **No OCI *import* path — prototyped, verdict: distraction** (branch `track/ocimport`,
  2026-07-28). The mechanical import is cheap and was proven: offline docker-archive/OCI-layout
  → deterministic store item + generated spec; real nginx and redis images ran under full
  hardening via `RootDirectory=`. But honest compatibility means a second full-rootfs runtime
  model (mutable-path inference, UID semantics, logging adaptation, a long tail) — not worth
  it. Kept open: a read-only `cix migrate` that extracts image metadata into a native Cixfile
  skeleton, reusing the parsing without runtime promises. Measured: ~25 MiB compressed OCI
  nginx → ~65 MiB rootfs item (Evidence-we-owe datapoint).
- **Networking between networked services.** Two apps that both need the network share the host
  stack: no per-app netns, no service DNS, no port inventory or collision management. A v3
  listener can bind an address through `cix run -p name=addr`, but ordinary declared ports still
  have no policy-level bind-address control. This is *the* design debt the compose era must pay first
  (part 3/part 5); until then the ledger's networking rows are honest IOUs.
- **The operational verb set is thin.** No `cix logs`, `inspect`, `stats`, `exec`, `wait`; no
  status/exit-cause view; stop/restart go through raw `systemctl`. Each is small and
  systemd-backed — this is roadmap material, listed per-row below as ❓, not a structural
  problem, but today an operator lives in two vocabularies.
- **Security posture is asserted, not published.** The sandbox is real, but "stricter than
  docker" needs receipts: publish the effective syscall/mount policy and grow a compatibility
  corpus as cixpkgs grows. Tracked in "Evidence we owe."

## Evidence we owe

Claims made elsewhere in this ledger that need measurements or documents before they count:

- Cold-transfer bytes and shared-content dedup: one prototype datum is ~25 MiB compressed OCI
  nginx → ~65 MiB *imported full-rootfs* item (`track/ocimport`), not a native sparse item;
  native items are kilobytes plus their shared store closure. Measure representative native
  services via Nix binary cache vs Docker compressed layers.
- The effective writable-mount and namespace set of a hardened unit, verified against docker's
  `--read-only` + default seccomp/userns behavior (incl. whether lacking a user namespace
  changes the containment boundary claim in §9).
- The `nix copy` offline round trip written up as the `save`/`load` equivalent (what metadata
  travels, what doesn't).
- Multi-arch end to end: build, serve, and pull every claimed system (D14 is metadata until
  then).
- Reproducibility enforcement: Cixfile v1 verifies the locked nixpkgs `narHash`
  (`crates/cix-cixfile/tests/lock_nix.rs`); rebuild verification remains, and nix *permits*
  impure derivations the product must refuse.
- The exact digest-semantics story: which docker `@sha256` properties a store path does and
  does not provide (D12).

## 1. Images, naming & distribution (fine—part 1 built)

| docker | disposition | still missing |
| --- | --- | --- |
| [image (artifact)](https://docs.docker.com/get-started/docker-concepts/the-basics/what-is-an-image/) | ✅ spec'd store item; D22 projects its sparse rootfs fragments read-only at native paths (`crates/cix-run/tests/system_projection.rs`) | Pulling or running existing OCI/Docker images; composix distributes only Nix closures described by its own entries. |
| [tag (mutable pointer)](https://docs.docker.com/reference/cli/docker/image/tag/) | ✅ `cix tag` (D5, D7: tags are GC roots) | — |
| [digest (`@sha256:…`)](https://docs.docker.com/dhi/core-concepts/digests/) | ✅ the store path *is* the digest (D12); no `@` syntax needed | — |
| [registry + pull](https://docs.docker.com/reference/cli/docker/image/pull/) | ✅ `cix serve` / `cix pull` (D6, D17) | OCI registry interoperability. |
| [push](https://docs.docker.com/reference/cli/docker/image/push/) | ⏳ deliberate (D17): later = “ask a server to publish for you,” ssh transport first | Publishing and established registry authentication today. |
| [default registry (`docker.io`)](https://docs.docker.com/docker-hub/) | ❌ by design: bare names are always local (D12) | Zero-configuration access to a shared public namespace. |
| [official images (`library/`)](https://docs.docker.com/docker-hub/image-library/trusted-content/) | 🔁 cixpkgs (planned; `examples/` is the seed) | — |
| [multi-platform images / manifest lists](https://docs.docker.com/build/building/multi-platform/) | ✅ per-system outputs (D14) | — |
| [`docker login` / `logout` / registry auth](https://docs.docker.com/reference/cli/docker/login/) | ⏳ arrives with push; authorization is server-side (D17) | Registry authentication and credential lifecycle today. |
| [content trust / signing](https://docs.docker.com/engine/security/trust/) | ✅ Nix path signatures + `trustedKeys` in entries | ❓ Specify key rotation, revocation, delegation, policy enforcement, and unsigned-path behavior before claiming operational parity. |
| [`save`](https://docs.docker.com/reference/cli/docker/image/save/) / [`load`](https://docs.docker.com/reference/cli/docker/image/load/) (tar transport) | 🔁 `nix copy --to file://…/ssh://…` is native | Streaming tagged images through Docker-compatible `save`/`load`. |
| [`image ls`](https://docs.docker.com/reference/cli/docker/image/ls/) / [`image rm`](https://docs.docker.com/reference/cli/docker/image/rm/) | ❓ `cix ls` / `cix untag` plus Nix GC look adjacent | ❓ Decide whether tag inventory and indirect shared-store collection satisfy Docker's image-object lifecycle. |
| [`image inspect`](https://docs.docker.com/reference/cli/docker/image/inspect/) | 🔁 informative URL page (D18) + `cix ls -l`; a `cix inspect` ❓ | Much less runtime/config metadata than Docker inspect. |
| [layers / `image history`](https://docs.docker.com/reference/cli/docker/image/history/) | ❌ no layers; native items are sparse D22 fragments, provenance = `drvPath` + `nix log` | Layer-level transfer reuse, authoring history, and a ubiquitous debugging vocabulary; no representative native-item cold-transfer or shared-content comparison with compressed layers. |
| [build-cache export/import](https://docs.docker.com/build/cache/backends/) | 🔁 Nix binary caches provide remote build results | — |
| [`image prune` / dangling images](https://docs.docker.com/reference/cli/docker/image/prune/) | 🔁 `cix untag` + Nix GC; `cix prune` sugar ⏳ | — |
| [registry mirrors / pull-through cache](https://docs.docker.com/docker-hub/image-library/mirror/) | 🔁 `substituters` list in entries (D6) | ❓ A list of content sources is not a pull-through mirror with upstream fill, freshness checks, and cache lifecycle. |
| [registry HTTP API](https://docs.docker.com/reference/api/registry/latest/) | ✅ one negotiated URL space (D18) | The standard Docker Registry HTTP API. |
| [`docker manifest`](https://docs.docker.com/reference/cli/docker/manifest/) | ❓ D14 stores per-system outputs | ❓ No client surface to inspect, annotate, create, or push a multi-platform manifest. |

## 2. Running (fine — part 2 built)

| docker | disposition | still missing |
| --- | --- | --- |
| [`run`](https://docs.docker.com/reference/cli/docker/container/run/) | ✅ `cix run` (transient hardened unit) | — |
| [`-e` environment](https://docs.docker.com/reference/cli/docker/container/run/#env) | ✅ declared string environment + validated `-e` (D21) | — |
| [`-p` port publish](https://docs.docker.com/reference/cli/docker/container/run/#publish) | 🔁 D24 compiles declared ports to kernel-enforced `SocketBindAllow=`/`SocketBindDeny=`; v3 `cix run -p name=addr` binds a listener through a transient socket unit (D29; `crates/cix-run/tests/system_projection.rs`). This is stronger than Docker's metadata-only `EXPOSE`, but host networking has no NAT. | ❓ No Docker-style remapping/NAT, port inventory, or collision management; ordinary declared ports have no policy-level bind-address choice. |
| [`-v` / `--mount`](https://docs.docker.com/engine/storage/bind-mounts/) | 🔁 role dirs (state/cache/logs/config/run, D11 narrowed); D22 sparse item paths are read-only projections, while operator host-binds are ⏳ compose | Arbitrary mutable host mounts; D11 deliberately narrows mount compatibility. |
| [`--restart` policies](https://docs.docker.com/engine/containers/start-containers-automatically/) | ⏳ compose (systemd `Restart=` natively) | — |
| [`HEALTHCHECK` / health status](https://docs.docker.com/reference/dockerfile/#healthcheck) | ⏳ parsed today, wired in compose era | Health state today. |
| [`logs`](https://docs.docker.com/reference/cli/docker/container/logs/) | 🔁 journald; foreground `cix run` streams and `journalctl -u cix-*` works | ❓ No `cix logs`, stable selector, per-app retention contract, or logging-driver integration. |
| [`exec` (command in container)](https://docs.docker.com/reference/cli/docker/container/exec/) | ❓ no container, but the unit's namespaces exist | ❓ Would `cix exec` enter them, and under which identity/capabilities? |
| [`attach`](https://docs.docker.com/reference/cli/docker/container/attach/) | 🔁 journal streaming | stdin/TTY attachment. |
| [`stop`](https://docs.docker.com/reference/cli/docker/container/stop/) / [`kill`](https://docs.docker.com/reference/cli/docker/container/kill/) / signals | ✅ `systemctl stop`; custom stop signal/timeouts ⏳ (spec v3 candidate) | — |
| [`rm` / `--rm`](https://docs.docker.com/reference/cli/docker/container/rm/) | 🔁 transient units self-collect | — |
| [`ps`](https://docs.docker.com/reference/cli/docker/container/ls/) | ✅ `cix ps` | — |
| [`stats`](https://docs.docker.com/reference/cli/docker/container/stats/) / [`top`](https://docs.docker.com/reference/cli/docker/container/top/) | 🔁 cgroup accounting is available through `systemd-cgtop`; `cix stats` sugar ⏳ | — |
| [`update` (live resource limits)](https://docs.docker.com/reference/cli/docker/container/update/) | ⏳ compose era (limits are operator config) | Live resource updates. |
| [`cp`](https://docs.docker.com/reference/cli/docker/container/cp/) | ❓ role dirs are plain host paths | ❓ Would path documentation replace copying to/from the immutable item and private namespaces? |
| [`commit` (container → image)](https://docs.docker.com/reference/cli/docker/container/commit/) | ❌ fundamental: artifacts come from builds, never snapshots (purity) | Capturing a debugged or manually modified runtime as a distributable artifact; mutable root filesystem. |
| [`pause`](https://docs.docker.com/reference/cli/docker/container/pause/) / [`unpause`](https://docs.docker.com/reference/cli/docker/container/unpause/) | 🔁 `systemctl freeze/thaw` exists; sugar probably ❌ | — |
| [`--user` (pick uid)](https://docs.docker.com/reference/cli/docker/container/run/#user) | ❌ `DynamicUser` is the model; fixed UIDs are refused | Compatibility with images, mounted files, licenses, and protocols that require a known numeric identity. |
| [`--privileged`](https://docs.docker.com/reference/cli/docker/container/run/#privileged) | ❌ against the capability-spec thesis (D20a); operator overrides live in compose ❓ | Workloads needing broad device/kernel access and the common diagnostic escape hatch. |
| [`--init`](https://docs.docker.com/reference/cli/docker/container/run/#init) | ❌ systemd is the service manager | Docker's portable in-container child reaping/forwarding behavior under another runtime. |
| [interactive `-it` containers](https://docs.docker.com/reference/cli/docker/container/run/#foreground) | 🔁 out of scope for services; `nix run`/`nix shell` cover Nix-native one-offs | — |
| [`create`](https://docs.docker.com/reference/cli/docker/container/create/) / [`start`](https://docs.docker.com/reference/cli/docker/container/start/) (stopped containers) | ❌ transient units are run-or-nothing; persistent units arrive with compose | Prepare/inspect/start workflows and a durable stopped-object inventory. |
| [`restart`](https://docs.docker.com/reference/cli/docker/container/restart/) | 🔁 `systemctl restart`; policy ⏳ compose | — |
| [`checkpoint` (CRIU)](https://docs.docker.com/reference/cli/docker/checkpoint/) | ❌ niche, no systemd first-class support | Checkpoint/restore, live-migration building blocks, and stateful fast restart. |
| [resource flags (`--cpu-*`, `--memory-*`, `--blkio-*`, `--ulimit`, `--oom-*`)](https://docs.docker.com/reference/cli/docker/container/run/#runtime-constraints-on-resources) | ⏳ compose: slice/unit limits, systemd-native | Resource knobs today. |
| [namespace modes (`--ipc`, `--pid`, `--uts`)](https://docs.docker.com/reference/cli/docker/container/run/#ipc) | 🔁 systemd sandboxing covers some isolation; *sharing* modes ⏳ compose ❓ | Explicit namespace controls and sharing modes. |
| [`--device` / `--gpus` / `--device-cgroup-rule`](https://docs.docker.com/reference/cli/docker/container/run/#device) | ⏳ deliberate (spec v2 deferral): needs a dogfood case; `DeviceAllow=` exists | Device access today. |
| [`--read-only`](https://docs.docker.com/reference/cli/docker/container/run/#read-only) | ✅ `ProtectSystem=strict` by default; D22 projects sparse item paths read-only at native locations, stress-tested for host-dir shadowing, symlink escape, and 25 mounts (`crates/cix-run/tests/system_projection.rs`) | — |
| [`--shm-size`](https://docs.docker.com/reference/cli/docker/container/run/#shm-size) | ⏳ PostgreSQL already brushed `/dev/shm`; no direct systemd knob, needs design | Configurable shared-memory size. |
| [`--sysctl`](https://docs.docker.com/reference/cli/docker/container/run/#sysctl) | ❌ host policy; per-netns sysctls ⏳ networking era | Safe namespaced tuning required by some databases, proxies, and network appliances. |
| [`--name`](https://docs.docker.com/reference/cli/docker/container/run/#name) | 🔁 unit names are systematic (`cix-run-<svc>-<nonce>`; compose plans `cix-<comp>-<svc>`) | ❓ A random run nonce is not a user-chosen stable handle. |
| [`--hostname`, `--dns*`, `--add-host`, `--ip*`, `--mac-address`, `--network-alias`](https://docs.docker.com/reference/cli/docker/container/run/#network-settings) | ⏳ networking era, wholesale | Network identity and configuration today. |
| [`--group-add` (supplementary groups)](https://docs.docker.com/reference/cli/docker/container/run/#additional-groups) | ❓ adjacent to device access; no case yet | ❓ No case yet. |
| [Docker-machinery flags (`--cidfile`, `--detach-keys`, `--label*`, `--annotation`, `--cgroup-parent`, `--isolation`, `--runtime`, `--publish-all`, `--volumes-from`)](https://docs.docker.com/reference/cli/docker/container/run/#options) | ❌ no composix analog planned | Machine-readable identity handoff, selectable runtimes/isolation, metadata-based automation, cgroup placement, automatic publication, and volume sharing. |
| [`container inspect`](https://docs.docker.com/reference/cli/docker/container/inspect/) | ❓ `cix ps` only lists running units | ❓ Define how users obtain resolved environment, mounts, sandbox, ports, state paths, status, and exit cause. |
| [`container prune`](https://docs.docker.com/reference/cli/docker/container/prune/) | ❓ transient units should collect automatically | ❓ Persistent compose units and role-directory lifecycle have no equivalent cleanup contract yet. |
| [`docker debug`](https://docs.docker.com/reference/cli/docker/debug/) | ❓ Docker offers a toolbox shell even for slim images | ❓ No image shell or packaged debug-tool injection story. |

## 3. Building (part 4 assembly subset built)

| docker | disposition | still missing |
| --- | --- | --- |
| [Dockerfile](https://docs.docker.com/reference/dockerfile/) | 🔁 Cixfile v1 + `cix build`, with line-numbered parse errors (D4; `crates/cix-cixfile/src/{parser,build}.rs`) + always the `.nix` escape hatch | Not a Dockerfile frontend: no arbitrary Dockerfile compatibility, `FROM`, layers, or `RUN`; the `.nix` escape hatch remains a migration burden. |
| [`FROM` / base images](https://docs.docker.com/reference/dockerfile/#from) | 🔁 locked nixpkgs `${pkgs.*}` references + ecosystem builders; no layer inheritance | ❓ This only helps software packaged in Nix or newly packaged for it; it cannot consume an arbitrary base image. |
| [`RUN`](https://docs.docker.com/reference/dockerfile/#run) | ❌ imperative impure steps; blessed builders instead | The universal escape hatch that makes existing installation instructions and most Dockerfiles directly expressible; broad packageability is traded for discipline. |
| [`COPY`](https://docs.docker.com/reference/dockerfile/#copy) / [`ADD`](https://docs.docker.com/reference/dockerfile/#add) / [`.dockerignore`](https://docs.docker.com/build/concepts/context/#dockerignore-files) | 🔁 implemented Cixfile sibling-file `COPY`, `FILE`/`SCRIPT` heredocs, and `LINK` assembly (`crates/cix-cixfile/tests/lock_nix.rs`) | No Docker-compatible recursive copy, URL/tar `ADD`, or `.dockerignore` rules. |
| [`ENV`, `EXPOSE`, `VOLUME`, `ENTRYPOINT`, `CMD`, `WORKDIR`, `HEALTHCHECK`, `USER`, `LABEL`](https://docs.docker.com/reference/dockerfile/#overview) | 🔁 implemented Cixfile `SERVICE` blocks compile `ENV`, `EXEC`, declared `PORT`, and role dirs to `cix-spec.json`; `PORT` is an enforced grant, not metadata (D24). v3 `listeners` are a separate cix-spec/run contract (D29), not a Cixfile v1 directive. | No native `LABEL`, arbitrary `USER`, `WORKDIR`, or `HEALTHCHECK`; `EXEC` combines entrypoint/command and role dirs are not Docker volume objects. |
| [`ARG` / build args](https://docs.docker.com/reference/dockerfile/#arg) | ❓ | ❓ Decide how configurable builds coexist with pinned inputs and cache identity. |
| [multi-stage builds](https://docs.docker.com/build/building/multi-stage/) | 🔁 derivations compose naturally | ❓ Show how Cixfile users—not `.nix` authors—express private intermediate tools and selective copying. |
| [BuildKit secret/SSH mounts](https://docs.docker.com/build/building/secrets/) | ❓ private dependencies—Nix has netrc/access tokens | ❓ The non-leaking Cixfile and remote-builder story is unspecified. |
| [reproducible builds](https://docs.docker.com/build/ci/github-actions/reproducible-builds/) | 🔁 Cixfile v1 locks nixpkgs revision + `narHash`, and a tampered hash fails (`crates/cix-cixfile/tests/lock_nix.rs`) | Rebuild verification is still absent; Nix permits impure derivations and composix does not yet refuse them product-wide. |
| [Dockerfile here-documents](https://docs.docker.com/reference/dockerfile/#here-documents) | ✅ implemented Cixfile `FILE`/`SCRIPT <<EOF` heredocs (`crates/cix-cixfile/tests/lock_nix.rs`) | — |
| [`STOPSIGNAL`](https://docs.docker.com/reference/dockerfile/#stopsignal) | ⏳ spec v3 candidate (`KillSignal=`), with stop timeouts | — |
| [`SHELL`, `ONBUILD`, `MAINTAINER`, parser directives](https://docs.docker.com/reference/dockerfile/#parser-directives) | ❌ `SCRIPT` has a fixed shell; no image inheritance; no parser magic | Inherited downstream triggers, per-image shell choice, frontend versioning, and direct compatibility with those Dockerfiles. |
| [`RUN --mount=cache/bind/tmpfs`, `--network`, `--security`](https://docs.docker.com/reference/dockerfile/#run) | ❌ falls with `RUN`; Nix builders are the proposed answer | Concise per-step cache, secret-adjacent, network, and security controls familiar to BuildKit users. |
| [Buildx / Bake / builder management](https://docs.docker.com/reference/cli/docker/buildx/) | ❌ Nix is the builder; remote/multi-platform 🔁 Nix distributed builds | The Docker CLI's named builders, Bake graph, standard exporters, driver choices, and existing CI actions. |
| [`docker init`](https://docs.docker.com/reference/cli/docker/init/) | ❓ no generator exists | ❓ No generator for a Cixfile, spec, Nix wrapper, or compose migration skeleton. |
| [build attestations](https://docs.docker.com/build/metadata/attestations/) | ❓ `drvPath` is provenance metadata | ❓ No standard SBOM/provenance attestation emission or policy story. |

## 4. Storage (coarse)

| docker | disposition | still missing |
| --- | --- | --- |
| [named volumes (including service sharing)](https://docs.docker.com/engine/storage/volumes/) | ⏳ compose era; D22 sparse projections are immutable item paths, not volume objects (`crates/cix-run/tests/system_projection.rs`) | Named volume objects, cross-service attachment, copy-up/`volume-subpath`, shared volumes, backup/restore, quota, snapshots, and encryption. |
| [bind mounts (host paths in)](https://docs.docker.com/engine/storage/bind-mounts/) | ⏳ compose era, operator territory | Arbitrary mounts; today composix creates service-specific FHS role directories indirectly through systemd. |
| [tmpfs mounts](https://docs.docker.com/engine/storage/tmpfs/) | ✅ `PrivateTmp`; more ⏳ | ❓ A private `/tmp` and `/var/tmp` does not cover arbitrary tmpfs destinations, sizing, modes, or swap behavior. |
| [volume drivers / plugins](https://docs.docker.com/engine/extend/plugins_volume/) | ❌ filesystems are the host's business | Portable Compose declarations for remote, encrypted, cloud, clustered, and vendor storage backends; a driver interface. |
| [`volume prune`](https://docs.docker.com/reference/cli/docker/volume/prune/) / [`volume update`](https://docs.docker.com/reference/cli/docker/volume/update/) | 🔁 role dirs are plain host paths; lifecycle via `systemctl clean` + GC | ❓ Nix GC does not clean mutable role data, and no `cix` command inventories ownership or previews deletion; no safe mutable-data garbage collector. |
| [`volume create` / `inspect` / `ls` / `rm`](https://docs.docker.com/reference/cli/docker/volume/) | ❓ compose has not defined the named-volume object | ❓ Metadata, sharing, lifecycle, or CLI surface are undefined. |

## 5. Networking (coarse — the biggest *conscious* gap)

| docker | disposition | still missing |
| --- | --- | --- |
| [port publish / NAT](https://docs.docker.com/engine/network/port-publishing/) | 🔁 D24 makes each declared port a kernel-enforced bind grant; D29 listeners bind `-p name=addr` through transient socket units (`crates/cix-run/tests/system_projection.rs`) | No address translation, port inventory/collision manager, or per-service network namespace; ordinary declared ports have no policy-level bind-address control. |
| [bridge networks and service DNS](https://docs.docker.com/engine/network/drivers/bridge/) | ⏳ compose-era decision (per-slice netns? socket activation?) | Per-container namespaces, bridge networks, user-defined networks, embedded DNS, aliases, and IPAM. |
| [network isolation](https://docs.docker.com/engine/network/) | ✅ coarse today (no ports ⇒ `PrivateNetwork`); finer ⏳ | ❓ A binary no-network switch does not isolate one networked composix app from another or from host interfaces. |
| [overlay / multi-host networking](https://docs.docker.com/engine/network/drivers/overlay/) | ❌ now; k8s-lite ambitions much later | Encrypted multi-daemon service networks and the basis for multi-host orchestration. |
| [`--link` (legacy)](https://docs.docker.com/engine/network/links/) | ❌ legacy environment/hosts coupling is refused | Legacy environment/hosts coupling (minor: Docker recommends user-defined networks). |
| [`network create` / `connect` / `disconnect` / `inspect` / `ls` / `prune` / `rm`](https://docs.docker.com/reference/cli/docker/network/) | ❓ no composix network object | ❓ No lifecycle surface: create, inspect, connect/disconnect, list, prune, or remove. |
| [host, none, macvlan, ipvlan network drivers](https://docs.docker.com/engine/network/drivers/) | ❓ the ledger only discusses bridge and overlay | ❓ Decide whether these modes are unsupported, operator overrides, or future capabilities. |

## 6. Compose (v0 built; deliberate walls remain)

Compose v0 is a root-managed systemd activation path, not a Docker Compose compatibility
layer. It has a strict `compose.json`, deterministic resolution and generation, a per-composite
Nix profile, and `cix up`/`down`/`rollback`; see `crates/cix-compose/` and the end-to-end
[`examples/compose/stack/demo.sh`](../examples/compose/stack/demo.sh). `cix compose check` and
`cix compose diff` dry-build without activation and work without root; `up`, `down`, and
`rollback` operate the system manager and require root. Netns, scale, health, secrets, limits,
and a reconciler remain deferred.

| docker | disposition | still missing |
| --- | --- | --- |
| [Compose services](https://docs.docker.com/reference/compose-file/services/) | ✅ strict machine-format `compose.json` services resolve local tags/store paths, select item services, and apply declared env/listener bindings (`crates/cix-compose/src/{model,resolve,generation}.rs`; `examples/compose/stack/compose.json`) | Docker Compose YAML compatibility, multi-file merging/includes, `extends`, and broad Compose-field coverage. |
| [`depends_on` / ordering](https://docs.docker.com/reference/compose-file/services/#depends_on) | 🔁 v0 Unix `edges` create setup units and consumer `Requires=`/`After=` ordering while granting only declared runtime paths (`crates/cix-compose/src/generation.rs`; stack `database`/`http` edges) | Docker's health/completion conditions and restart propagation; network dependency/discovery waits for the networking work. |
| [scale / replicas](https://docs.docker.com/reference/cli/docker/compose/scale/) | ⏳ no replica field or template units in v0 | Scaling, stable replica identities, placement, and lifecycle semantics. |
| [`env_file` / secrets](https://docs.docker.com/reference/compose-file/services/#env_file) | 🔁 v0 has explicit string `env` overrides (`crates/cix-compose/src/model.rs`); `env_file` and credential delivery are deferred | File-based environment loading, secrets, and `LoadCredential=` integration. |
| [resource limits](https://docs.docker.com/reference/compose-file/deploy/#resources) | ⏳ no compose limits yet; future operator policy maps to systemd slice/unit properties | CPU, memory, IO, PID, and reservation controls. |
| [project namespacing](https://docs.docker.com/compose/how-tos/project-name/) | ✅ the compose name produces `cix-<name>.slice`/`.target`, `cix-<name>-<service>` units, and a `cix-compose-<name>` Nix profile (`crates/cix-compose/src/{generation,runtime}.rs`) | Docker's project-name flags and compatibility naming rules. |
| [`up`](https://docs.docker.com/reference/cli/docker/compose/up/) / [`down`](https://docs.docker.com/reference/cli/docker/compose/down/) / rollback | ✅ `cix up` resolves, writes `cix.lock`, builds/activates a profile; `cix down` unlinks managed units while retaining it; `cix rollback` activates the preceding generation (`crates/cix-compose/src/runtime.rs`; `examples/compose/stack/demo.sh`) | Rootless activation, a long-running reconciler, and Docker-compatible lifecycle flags/observation commands. |
| [`docker compose config`](https://docs.docker.com/reference/cli/docker/compose/config/) | 🔁 `cix compose check` resolves and semantically validates; `cix compose diff` dry-builds and compares the prospective generation without activation (`crates/cix-compose/src/{cli,runtime}.rs`) | A merged/canonical Compose configuration emitter, multi-file inputs, and Docker's config command/options. |
| [`watch` (dev mode)](https://docs.docker.com/compose/how-tos/file-watch/) | ❓ interesting dev loop, unscoped | ❓ Unscoped. |
| [profiles](https://docs.docker.com/reference/compose-file/profiles/) | 🔁 each composite has a Nix profile whose retained generations power rollback (`crates/cix-compose/src/runtime.rs`) | Docker Compose service-profile selection, dependency validation, and profile-aware locking are not implemented. |
| [`configs` top-level element](https://docs.docker.com/reference/compose-file/configs/) | ⏳ no compose config object in v0 | Top-level config objects, `ConfigurationDirectory=` materialization, ownership/mode, and attachment semantics. |
| [networks, volumes, secrets, configs as reusable top-level objects](https://docs.docker.com/reference/compose-file/) | ⏳ v0's named Unix edges are narrowly scoped runtime-path grants, not reusable Docker objects (`crates/cix-compose/src/{model,generation}.rs`) | Object identity, external resources, lifecycle, named volumes, networks, secrets, and configs. |
| [`version` marker (obsolete)](https://docs.docker.com/reference/compose-file/version-and-name/#version-top-level-element-obsolete) | ❌ | — |

## 7. Daemon & platform

| docker | disposition | still missing |
| --- | --- | --- |
| [`dockerd` (the daemon)](https://docs.docker.com/reference/cli/dockerd/) | ❌ systemd is the runtime; `cix` is a CLI + later a small reconciler (D9) | One versioned engine API owning lifecycle, images, networks, volumes, events, metrics, and remote automation; a unifying platform surface. |
| [`docker context` / remote hosts](https://docs.docker.com/reference/cli/docker/context/) | ❓ ssh is the transport today; `cix --host` sugar maybe ⏳ | ❓ Remote contexts. |
| [events API](https://docs.docker.com/reference/cli/docker/system/events/) | 🔁 journald/systemd events | Typed events API. |
| [logging drivers](https://docs.docker.com/engine/logging/configure/) | ❌ journald; forwarding is journald's job | Per-workload selection and portable Compose configuration for JSON, syslog, Fluentd, GELF, cloud, and plugin drivers. |
| [storage drivers](https://docs.docker.com/engine/storage/drivers/select-storage-driver/) | ❌ the Nix store | Platform/filesystem-specific runtime-layer choices and compatibility with Docker's mutable container filesystems. |
| [plugins](https://docs.docker.com/engine/extend/) | ❌ no plugin interface | Third-party volume, network, authorization, logging, and other daemon extensions. |
| [rootless mode](https://docs.docker.com/engine/security/rootless/) | 🔁 `--user` degraded dev mode exists (D13); full rootless is not a goal | A materially complete rootless mode. |
| [Docker Desktop / GUI](https://docs.docker.com/desktop/) | ❌ no desktop product | Supported macOS/Windows development, managed VM/updates, file sharing, proxy/VPN handling, credential integration, GUI diagnostics, extensions, and optional Kubernetes. |
| [`system df` / `info` / `prune`](https://docs.docker.com/reference/cli/docker/system/) | ❓ | ❓ No unified disk-usage, capability/status, or safe unused-resource cleanup view. |
| [Docker Engine API and SDKs](https://docs.docker.com/reference/api/engine/) | ❓ D9 mentions a reconciler | ❓ No stable local/remote API, compatibility policy, or client libraries. |
| [CLI configuration, credential stores, proxies, TLS](https://docs.docker.com/reference/cli/docker/#configuration-files) | ❓ the index has signing/auth fields | ❓ No complete client configuration or credential-management model. |
| [Docker Offload](https://docs.docker.com/offload/) | ❓ | ❓ Decide whether cloud build/run offload is rejected, delegated to Nix builders, or a future remote-runtime feature. |

## 8. Orchestration (swarm)

| docker | disposition | still missing |
| --- | --- | --- |
| [Swarm mode](https://docs.docker.com/engine/swarm/) | ❌ wholesale; composix is single-host | Clustered desired state, managers/workers, scheduling, reconciliation, rolling updates, service discovery, mutual TLS, and failure rescheduling. |
| [`docker node`](https://docs.docker.com/reference/cli/docker/node/) | ❌ no cluster membership, availability, labels, promotion/demotion, drain, or node inspection | Encrypted manager/worker membership and node lifecycle. |
| [`docker service`](https://docs.docker.com/reference/cli/docker/service/) | ❌ no replicated/global service object, placement, update/rollback, endpoint, or service logs | Placement, service VIP/DNS, host-failure recovery, and workload rescheduling. |
| [`docker stack`](https://docs.docker.com/reference/cli/docker/stack/) | ❌ no multi-node Compose deployment or stack lifecycle | Multi-node Compose deployment and stack lifecycle. |
| [Swarm configs](https://docs.docker.com/reference/cli/docker/config/) | ❌ no cluster config distribution; compose-level local configuration is only deferred | Cluster config distribution. |
| [Swarm secrets](https://docs.docker.com/reference/cli/docker/secret/) | ❌ no encrypted cluster secret distribution; local `LoadCredential=` delivery is only deferred | Encrypted cluster secret distribution. |

## 9. Security

| docker | disposition | still missing |
| --- | --- | --- |
| [seccomp profiles](https://docs.docker.com/engine/security/seccomp/) | ✅ `SystemCallFilter=@system-service` default; custom profiles ❌ (D20a) | Years of deployment exposure and published seccomp behavior; composix has no published threat model, audit, or compatibility corpus. |
| [capabilities (`--cap-add`)](https://docs.docker.com/engine/containers/run/#runtime-privilege-and-linux-capabilities) | ✅ semantic grants only: declared ports compile to kernel-enforced `SocketBindAllow=`/`SocketBindDeny=` (and <1024 ⇒ `NET_BIND_SERVICE`); undeclared bind denial is live-tested in `crates/cix-run/tests/system_projection.rs`. Docker has no equivalent port declaration enforcement. | ❓ One implemented semantic grant is not coverage of real workloads that need other narrowly scoped capabilities. |
| [userns-remap](https://docs.docker.com/engine/security/userns-remap/) | ✅ `DynamicUser` + idmapped mounts | ❓ Services do not run in a user namespace—the containment boundary differs from Docker's userns; verify before claiming parity (see Evidence). |
| [AppArmor](https://docs.docker.com/engine/security/apparmor/) / [SELinux](https://docs.docker.com/engine/storage/bind-mounts/#configure-the-selinux-label) | ❓ host policy, likely out of spec scope | ❓ Define labeling/profile behavior for store items and managed writable directories. |
| [secrets](https://docs.docker.com/compose/how-tos/use-secrets/) | ⏳ `LoadCredential=`, compose era | Secret delivery today. |
| [SBOM / vulnerability scanning (Scout)](https://docs.docker.com/scout/) | 🔁 the closure is an exact dependency inventory; tooling ⏳ | ❓ A closure is not SPDX/CycloneDX; there is no CVE matcher, standard SBOM, or registry-integrated scanner. |
| [provenance / attestations](https://docs.docker.com/build/metadata/attestations/slsa-provenance/) | 🔁 `drvPath` + path signatures carry real provenance; a standard exchange format (SLSA-shaped) ⏳ | Standard provenance/attestation exchange format. |
| [authorization plugins](https://docs.docker.com/engine/extend/plugins_authorization/) | ❓ rejecting plugins | ❓ No stated policy-enforcement point for a future server/reconciler API. |
| [Docker Desktop Enhanced Container Isolation](https://docs.docker.com/enterprise/security/hardened-desktop/enhanced-container-isolation/) | ❓ | ❓ No comparable VM/user-namespace boundary or enterprise policy/control plane; decide whether that threat model is out of scope. |

## 10. Hub & ecosystem

| docker | disposition | still missing |
| --- | --- | --- |
| [Hub search / explore](https://docs.docker.com/docker-hub/image-library/) | ⏳ the serve pages (D18) are the seed of “explore” | Millions of discoverable images, trusted catalogs, CI/IDE/SDK and Testcontainers integrations, and a large installed community; composix has one HTML listing and two examples. |
| [`docker dhi` (hardened images)](https://docs.docker.com/reference/cli/docker/dhi/) / [`docker model` (AI artifacts)](https://docs.docker.com/reference/cli/docker/model/) | ❌ product catalog features, not runtime concepts | Docker's maintained hardened supply chain and turnkey local model packaging/execution. |
| [automated builds](https://docs.docker.com/docker-hub/repos/manage/builds/) | ❌ CI's job | Repository-integrated source triggers, build rules, status, and a hosted path from commit to published artifact. |
| [misc CLI: `diff`](https://docs.docker.com/reference/cli/docker/container/diff/), [`export`](https://docs.docker.com/reference/cli/docker/container/export/), [`import`](https://docs.docker.com/reference/cli/docker/image/import/), [`rename`](https://docs.docker.com/reference/cli/docker/container/rename/) | ❌ artifacts are immutable store items | Runtime filesystem diff/export, rootfs import, and mutable container handles. |
| [`wait`](https://docs.docker.com/reference/cli/docker/container/wait/), [`port`](https://docs.docker.com/reference/cli/docker/container/port/), [`version`](https://docs.docker.com/reference/cli/docker/version/), [`info`](https://docs.docker.com/reference/cli/docker/system/info/) | 🔁 trivial equivalents where useful | — |
| [`docker search`](https://docs.docker.com/reference/cli/docker/search/) | ❓ D18 serves one index's listing | ❓ No federated/catalog search, ranking, trust metadata, or discovery across indexes. |
| [Hub organizations, access, webhooks, and repository management](https://docs.docker.com/docker-hub/repos/) | ❓ D17 defers server authorization | ❓ No teams, roles, private repositories, audit, lifecycle policies, or event integration. |
| [`docker scout`](https://docs.docker.com/reference/cli/docker/scout/) | ❓ closure metadata could feed future analysis | ❓ No vulnerability, policy, comparison, recommendation, or remediation command. |
| [`docker mcp`](https://docs.docker.com/reference/cli/docker/mcp/) | ❓ newly present in the Docker CLI index | ❓ Decide whether MCP catalog/toolkit integration is irrelevant, external, or a composix ecosystem concern. |
| [`docker pass`](https://docs.docker.com/reference/cli/docker/pass/) | ❓ newly present in the Docker CLI index | ❓ No local OS-keychain secret-management surface. |

The [top-level Docker CLI reference](https://docs.docker.com/reference/cli/docker/) was used as
the thoroughness checklist. Concepts without an existing composix decision are marked ❓ rather
than assigned a disposition here.

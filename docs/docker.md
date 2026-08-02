# The Docker ledger

Every Docker concept gets a *conscious* disposition—adopted, adapted, rejected, or deferred—
nothing by accident. This is a ledger against Docker as it exists, not against a convenient
subset of it. “Designed” and “provided by Nix or systemd” do not mean “shipped by composix.”
Every ❓ goes through Mathijs before it becomes a decision. Dispositions cite `design.md`
decisions where they exist.

Legend: ✅ have · 🔁 adapted (solved differently) · ❌ rejected · ⏳ deferred (with a
target era) · ❓ needs discussion. The third column names what Docker still provides that
composix does not.

Terminology note (D33): composix's `cix-manifest.json` corresponds to what OCI calls the
image **config** (`vnd.oci.image.config.v1+json`) — in OCI, "manifest" is the registry-side
descriptor listing layers. We say *manifest* anyway: it's what the docker world colloquially
calls the baked metadata, and "config" in composix means operator territory (D20b).

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
  → deterministic store item + generated manifest; real nginx and redis images ran under full
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
- **Health follows its consumers.** CIP-79's `READINESS`/`LIVENESS` directives now compile
  to systemd start-job readiness and watchdog restart, with native cix HTTP/TCP adapters.
  A separate health-gated dependency graph is explicitly rejected; structural edges wait
  for readiness through ordinary systemd ordering.
- **Directory declarations now carry deployment materialization.** CIP-82 maps declared
  paths through compose `host:`/`shared:`/`as:` entries, resolves interpolation only from
  the compose directory's `.env`, and gives cache/log/state a deliberate clean/purge contract.
  `DIR` still has no private default: it requires operator host or shared backing.
- **The operational verb set is deliberately a projection.** `cix logs`, `ps`, `inspect`, and
  `stats` read journald/systemd state; they hold no parallel database or daemon. Each prints or
  documents its raw equivalent, so operators can move freely between the cix and systemd
  vocabularies.
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
| [image (artifact)](https://docs.docker.com/get-started/docker-concepts/the-basics/what-is-an-image/) | ✅ manifested store item; D22 projects its sparse rootfs fragments read-only at native paths (`crates/cix-run/tests/system_projection.rs`) | Pulling or running existing OCI/Docker images; composix distributes only Nix closures described by its own entries. |
| [tag (mutable pointer)](https://docs.docker.com/reference/cli/docker/image/tag/) | ✅ `cix tag`; Cixfile member names are declared while `-t <tag>` applies external family tags without rebuilding (D5, D7, D62; tags are GC roots). An anonymous `cix run` roots its item only for the lifetime of its unit (D63). | No implicit `latest`; every ref spells its tag. |
| [digest (`@sha256:…`)](https://docs.docker.com/dhi/core-concepts/digests/) | ✅ the store path *is* the digest (D12); no `@` syntax needed | — |
| [registry + pull](https://docs.docker.com/reference/cli/docker/image/pull/) | ✅ `cix serve` / `cix pull` (D6, D17) | OCI registry interoperability. |
| [registry mirrors / pull-through cache](https://docs.docker.com/docker-hub/image-library/mirror/) | 🔁 no mirror feature (D35): bytes — substituters are the mirror surface (entries list multiple locations; content-bound signatures make untrusted mirrors harmless); index availability — plain HTTP caching/CDN, since the API is ordinary content-negotiated GET (D18) | Independent index *redistribution* ⏳ publish-era, gated on entry signing (D35). |
| [push](https://docs.docker.com/reference/cli/docker/image/push/) | ⏳ deliberate (D17): later = “ask a server to publish for you,” ssh transport first | Publishing and established registry authentication today. |
| [default registry (`docker.io`)](https://docs.docker.com/docker-hub/) | ❌ by design: bare names are always local (D12) | Zero-configuration access to a shared public namespace. |
| [official images (`library/`)](https://docs.docker.com/docker-hub/image-library/trusted-content/) | 🔁 cixpkgs (planned; `examples/` is the seed) | — |
| [multi-platform images / manifest lists](https://docs.docker.com/build/building/multi-platform/) | ✅ per-system outputs (D14) | — |
| [`docker login` / `logout` / registry auth](https://docs.docker.com/reference/cli/docker/login/) | ⏳ arrives with push; authorization is server-side (D17) | Registry authentication and credential lifecycle today. |
| [content trust / signing](https://docs.docker.com/engine/security/trust/) | ✅ content signing: Nix path signatures + `trustedKeys` are content-bound — trust travels with the bytes regardless of serving cache (D35). Entry (tag→path) trust today = TLS to origin, i.e. docker-without-DCT parity (DCT adoption was ~nil; the ecosystem moved to sigstore) | ⏳ publish-era (D35): entry signing + key rotation/revocation/delegation/policy — needed exactly when third parties sit between origin and consumer. |
| [`save`](https://docs.docker.com/reference/cli/docker/image/save/) / [`load`](https://docs.docker.com/reference/cli/docker/image/load/) (tar transport) | 🔁 `nix copy --to file://…/ssh://…` is native | Streaming tagged images through Docker-compatible `save`/`load`. |
| [`image ls`](https://docs.docker.com/reference/cli/docker/image/ls/) / [`image rm`](https://docs.docker.com/reference/cli/docker/image/rm/) | 🔁 the tag is the object (D35): `cix ls` / `cix untag`; store items are shared substrate, nix GC collects whatever is unrooted — no dangling-image objects or prune verb by design; at most a `nix store gc` disk-space hint | — |
| [`image inspect`](https://docs.docker.com/reference/cli/docker/image/inspect/) | ✅ `cix inspect <ref>` (D35): stable JSON (or `--human`) reports store path, narHash, per-system outputs, validated parsed manifest, closure size, recorded trusted keys, upstream, and drvPath when present. | — |
| [layers / `image history`](https://docs.docker.com/reference/cli/docker/image/history/) | ❌ no layers; native items are sparse D22 fragments, provenance = `drvPath` + `nix log` | Layer-level transfer reuse, authoring history, and a ubiquitous debugging vocabulary; no representative native-item cold-transfer or shared-content comparison with compressed layers. |
| [build-cache export/import](https://docs.docker.com/build/cache/backends/) | 🔁 Nix binary caches provide remote build results | — |
| [`image prune` / dangling images](https://docs.docker.com/reference/cli/docker/image/prune/) | 🔁 `cix untag` + Nix GC; `cix prune` sugar ⏳ | — |
| [registry HTTP API](https://docs.docker.com/reference/api/registry/latest/) | ✅ one negotiated URL space (D18) | The standard Docker Registry HTTP API. |
| [`docker manifest`](https://docs.docker.com/reference/cli/docker/manifest/) | 🔁 no verb needed (D35): entries are natively per-system (D14), so create/annotate/push have no meaning; `cix ls -l` has a systems column and `cix inspect` exposes the output map. | — |

## 2. Running (fine — part 2 built)

| docker | disposition | still missing |
| --- | --- | --- |
| [`run`](https://docs.docker.com/reference/cli/docker/container/run/) | ✅ `cix run` (transient hardened unit) | — |
| [`-e` environment](https://docs.docker.com/reference/cli/docker/container/run/#env) | ✅ declared string environment + validated `-e` (D21) | — |
| [`-p` port publish](https://docs.docker.com/reference/cli/docker/container/run/#publish) | 🔁 D24 compiles declared ports to kernel-enforced `SocketBindAllow=`/`SocketBindDeny=`; v3 `cix run -p name=addr` binds a listener through a transient socket unit (D29; `crates/cix-run/tests/system_projection.rs`). This is stronger than Docker's metadata-only `EXPOSE`, but host networking has no NAT. | ❓ No Docker-style remapping/NAT, port inventory, or collision management; ordinary declared ports have no policy-level bind-address choice. |
| [`-v` / `--mount`](https://docs.docker.com/engine/storage/bind-mounts/) | ✅ cix-managed private data is a role dir; `DIR` is supplied through strict compose `host:` or `shared:` backing, and `as:` changes only its lifecycle treatment. `cix run --dir` mirrors host/reclassification flags. | No Docker mount grammar, copy-up, anonymous volumes, or mount propagation vocabulary. |
| [`--restart` policies](https://docs.docker.com/engine/containers/start-containers-automatically/) | 🔁 declaring `LIVENESS` is an explicit restart opt-in: cix emits a fixed bounded `Restart=on-failure`/`RestartSec=`/start-limit policy around systemd's watchdog | No Docker policy-name compatibility or per-service restart tuning. |
| [`HEALTHCHECK` / health status](https://docs.docker.com/reference/dockerfile/#healthcheck) | ✅ CIP-79 splits the consumers into `READINESS` and `LIVENESS`; native notify or cix-owned HTTP/TCP probes compile to systemd readiness/watchdog behavior, and the health VM proves rollout failure plus watchdog recovery | No generic exec probe or standalone health-status bit. Docker's `condition: service_healthy` graph is deliberately rejected; structural edges wait for readiness. |
| [`logs`](https://docs.docker.com/reference/cli/docker/container/logs/) | 🔁 `cix logs <compose>[/<service>]` is an indexed `journalctl CIX_COMPOSITE=… CIX_SERVICE=…` projection; `-f`, `--since`, `-n`, and `--invocation` translate directly. `cix logs --explain` prints the raw command. | No stdin/TTY attachment or separate cix log store. |
| [`exec` (command in container)](https://docs.docker.com/reference/cli/docker/container/exec/) | ✅ `cix exec` joins whichever of a running unit's mount/PID/network/IPC/UTS namespaces are private and reconstructs its recorded environment; system-mode services have a private PID namespace by default, while network remains host-shared for network-enabled services (D34, D36) | Defaults to the service UID/GID, with explicit `--root`. `PrivatePIDs` is dropped loudly on unsupported systemd/user-manager degraded paths, so the exec banner remains the source of truth. Deliberately no synthetic service seccomp/capability/cgroup confinement: this is operator surgery, not another service process (D34). |
| [`attach`](https://docs.docker.com/reference/cli/docker/container/attach/) | 🔁 journal streaming | stdin/TTY attachment. |
| [`stop`](https://docs.docker.com/reference/cli/docker/container/stop/) / [`kill`](https://docs.docker.com/reference/cli/docker/container/kill/) / signals | ✅ `systemctl stop`; custom stop signal/timeouts ⏳ (future manifest field) | — |
| [`rm` / `--rm`](https://docs.docker.com/reference/cli/docker/container/rm/) | 🔁 transient units self-collect | — |
| [`ps`](https://docs.docker.com/reference/cli/docker/container/ls/) | ✅ `cix ps` includes the native systemd RESULT (including the watchdog diagnosis) | Docker's container IDs and separate health status. |
| [`stats`](https://docs.docker.com/reference/cli/docker/container/stats/) / [`top`](https://docs.docker.com/reference/cli/docker/container/top/) | 🔁 `cix stats` provides one accounting snapshot (memory, CPU, tasks, IO/IP); live observation is `systemd-cgtop`. | Docker's streaming interactive table and process `top` view. |
| per-app log retention | 🔁 compose `logNamespace: true` gives `cix-<compose>` its own journald namespace and retention configuration | Per-service policy fields; namespace creation is an opt-in operational shift, not cix-managed retention. |
| [`update` (live resource limits)](https://docs.docker.com/reference/cli/docker/container/update/) | ⏳ compose era (limits are operator config) | Live resource updates. |
| [`cp`](https://docs.docker.com/reference/cli/docker/container/cp/) | ❓ `cix inspect --runtime` exposes item and host role-dir paths | ❓ There is no copy verb across the immutable item/private namespace boundary; decide whether explicit paths are sufficient. |
| [`commit` (container → image)](https://docs.docker.com/reference/cli/docker/container/commit/) | ❌ fundamental: artifacts come from builds, never snapshots (purity) | Capturing a debugged or manually modified runtime as a distributable artifact; mutable root filesystem. |
| [`pause`](https://docs.docker.com/reference/cli/docker/container/pause/) / [`unpause`](https://docs.docker.com/reference/cli/docker/container/unpause/) | 🔁 `systemctl freeze/thaw` exists; sugar probably ❌ | — |
| [`--user` (pick uid)](https://docs.docker.com/reference/cli/docker/container/run/#user) | ❌ `DynamicUser` is the model; fixed UIDs are refused | Compatibility with images, mounted files, licenses, and protocols that require a known numeric identity. |
| [`--privileged`](https://docs.docker.com/reference/cli/docker/container/run/#privileged) | ❌ against the capability-contract thesis (D20a); operator overrides live in compose ❓ | Workloads needing broad device/kernel access and the common diagnostic escape hatch. |
| [`--init`](https://docs.docker.com/reference/cli/docker/container/run/#init) | ❌ systemd is the service manager | Docker's portable in-container child reaping/forwarding behavior under another runtime. |
| [interactive `-it` containers](https://docs.docker.com/reference/cli/docker/container/run/#foreground) | 🔁 out of scope for services; `nix run`/`nix shell` cover Nix-native one-offs | — |
| [`create`](https://docs.docker.com/reference/cli/docker/container/create/) / [`start`](https://docs.docker.com/reference/cli/docker/container/start/) (stopped containers) | ❌ transient units are run-or-nothing; persistent units arrive with compose | Prepare/inspect/start workflows and a durable stopped-object inventory. |
| [`restart`](https://docs.docker.com/reference/cli/docker/container/restart/) | 🔁 `systemctl restart`; policy ⏳ compose | — |
| [`checkpoint` (CRIU)](https://docs.docker.com/reference/cli/docker/checkpoint/) | ❌ niche, no systemd first-class support | Checkpoint/restore, live-migration building blocks, and stateful fast restart. |
| [resource flags (`--cpu-*`, `--memory-*`, `--blkio-*`, `--ulimit`, `--oom-*`)](https://docs.docker.com/reference/cli/docker/container/run/#runtime-constraints-on-resources) | ⏳ compose: slice/unit limits, systemd-native | Resource knobs today. |
| [namespace modes (`--ipc`, `--pid`, `--uts`)](https://docs.docker.com/reference/cli/docker/container/run/#ipc) | 🔁 system-mode services get a private PID namespace by default (`PrivatePIDs=yes`, D36); systemd sandboxing covers some other isolation, while *sharing* modes ⏳ compose ❓ | Explicit namespace controls and sharing modes. |
| [`--device` / `--gpus` / `--device-cgroup-rule`](https://docs.docker.com/reference/cli/docker/container/run/#device) | ✅ `CLAIM device /dev/<node>` and `CLAIM gpu` compile to a closed `DevicePolicy=` allow-list. The former resolves the node group at generation time; the latter allows `/dev/dri` and adds `video render`. No `--privileged`, compose device grants, CDI, or GPU driver-library injection. | Multi-device/vendor allocation and operator widening remain intentionally unbuilt. |
| [`--read-only`](https://docs.docker.com/reference/cli/docker/container/run/#read-only) | ✅ CIP-84's `--closed-root` audit runtime uses an empty `RootDirectory`, whole-store ro, and only D22/role/claim projections; `ProtectSystem=strict` remains defense in depth. Unit snapshots plus the closed-root VM audit cover the sealed visibility boundary. | Phase 1 keeps the flag off by default while the artifact tier is audited; phase 2 removes the flag and makes this the only runtime. |
| [`--shm-size`](https://docs.docker.com/reference/cli/docker/container/run/#shm-size) | ✅ `SHM <size>` compiles to a private `TemporaryFileSystem=/dev/shm:size=…`; compose service `shm:` overrides it and is shown by `cix compose diff`. | No generic resource-policy surface. |
| [`--sysctl`](https://docs.docker.com/reference/cli/docker/container/run/#sysctl) | ❌ host policy; per-netns sysctls ⏳ networking era | Safe namespaced tuning required by some databases, proxies, and network appliances. |
| [`--name`](https://docs.docker.com/reference/cli/docker/container/run/#name) | 🔁 unit names are systematic (`cix-run-<svc>-<nonce>`; compose plans `cix-<comp>-<svc>`) | ❓ A random run nonce is not a user-chosen stable handle. |
| [`--hostname`, `--dns*`, `--add-host`, `--ip*`, `--mac-address`, `--network-alias`](https://docs.docker.com/reference/cli/docker/container/run/#network-settings) | ⏳ networking era, wholesale | Network identity and configuration today. |
| [`--group-add` (supplementary groups)](https://docs.docker.com/reference/cli/docker/container/run/#additional-groups) | 🔶 device claims resolve the necessary owning group; `CLAIM gpu` adds `video render`. | Arbitrary group addition remains refused: groups are device-access implementation detail, not a broad user knob. |
| [Docker-machinery flags (`--cidfile`, `--detach-keys`, `--label*`, `--annotation`, `--cgroup-parent`, `--isolation`, `--runtime`, `--publish-all`, `--volumes-from`)](https://docs.docker.com/reference/cli/docker/container/run/#options) | ❌ no composix analog planned | Machine-readable identity handoff, selectable runtimes/isolation, metadata-based automation, cgroup placement, automatic publication, and volume sharing. |
| [`container inspect`](https://docs.docker.com/reference/cli/docker/container/inspect/) | ✅ `cix inspect <unit-or-service>` (D35): state, MainPID, last exit cause, effective generated properties, port/listener bindings, and host role-dir paths; ambiguous tag/service names require an explicit world flag. | — |
| [`container prune`](https://docs.docker.com/reference/cli/docker/container/prune/) | 🔁 transient units self-collect; CIP-82 defines role-dir lifecycle and refuses container recreation as meaningless | `cix clean` and composite purge are not implemented yet; no inventory/preview command exists. |
| [`docker debug`](https://docs.docker.com/reference/cli/docker/debug/) | ✅ `cix debug` creates a fresh, fully sandboxed transient unit for a shell/command; `cix exec` joins a live unit's private namespaces with its recorded environment. Both resolve tools through the service PATH followed by `/usr/bin:/bin` (D34; D31 addendum) | Future `--with <pkg>` toolbox injection. Persistent pet-server shell workflows are consciously refused: debug units are fresh and `--collect`; live exec is explicit operator surgery (D34). |

## 3. Building (part 4 assembly subset built)

| docker | disposition | still missing |
| --- | --- | --- |
| [Dockerfile](https://docs.docker.com/reference/dockerfile/) | 🔁 Cixfile v1 + `cix build`, with D47's line-numbered block grammar: named `BUILDER`s feed explicit `SERVICE` and `APP` artifacts through binders, and project-local `FROM … OVERLAY` customizes a package universe (D4/D47/D50/D70; `crates/cix-cixfile/src/{parser,build_chain}.rs`) + the `.nix` escape hatch | Not an arbitrary Dockerfile frontend: `FROM` binds package universes or source trees rather than filesystem inheritance; org-wide universe trees still use the escape hatch. |
| [`FROM` / base images](https://docs.docker.com/reference/dockerfile/#from) | 🔁 `FROM <flakeref> AS <name>` binds a locked package universe or remote source tree; `FROM . AS src` optionally names the unpinned local Cixfile context; no layer inheritance | ❓ This only helps software packaged in Nix or built from source; it cannot consume an arbitrary base image. |
| [`RUN`](https://docs.docker.com/reference/dockerfile/#run) | ✅ D47/D51/D71/CIP-87: builder-only, sandboxed, networkless `RUN`, with one-line and heredoc forms; traced regular reads, directory listings, and absent-path probes provide constructive-trace memo hits independent of unrelated workspace changes. Networked `FETCH` uses the same read-set cutoff plus output pins, while each builder's disposable underlay preserves incremental tool state; `--cold` audits reads and outputs from empty. | Persistent workspaces and step-output snapshots are host-local and disposable; tracing currently requires Linux ptrace/`strace`. |
| [`COPY`](https://docs.docker.com/reference/dockerfile/#copy) / [`ADD`](https://docs.docker.com/reference/dockerfile/#add) / [`.dockerignore`](https://docs.docker.com/build/concepts/context/#dockerignore-files) | 🔁 one `COPY` moves bare local-context paths or `${binder}/…` paths into the current builder/artifact; directory COPY is preferred, scripts remain real copied files, while interpolated `FILE` and target-first `LINK` assemble additional content | No URL/tar auto-extracting `ADD`, structural globs, or `.dockerignore` rules. |
| [`ENV`, `EXPOSE`, `VOLUME`, `ENTRYPOINT`, `CMD`, `WORKDIR`, `HEALTHCHECK`, `USER`, `LABEL`](https://docs.docker.com/reference/dockerfile/#overview) | 🔁 `VOLUME` splits honestly: use `STATEDIR` (or another role) for cix-managed private data, and `DIR` for operator-supplied content. Each `SERVICE` or `APP` compiles one bare v0 manifest; `PORT` is an enforced capability declaration, not metadata; `READINESS`/`LIVENESS` replace exec-shaped `HEALTHCHECK`. | `DIR` compose materialization remains pending; no generic exec health probe, native `LABEL`, arbitrary `USER`, or `WORKDIR`. |
| [`ARG` / build args](https://docs.docker.com/reference/dockerfile/#arg) | ❓ | ❓ Decide how configurable builds coexist with pinned inputs and cache identity. |
| [multi-stage builds](https://docs.docker.com/build/building/multi-stage/) | ✅ multiple named `BUILDER` blocks chain through `COPY ${prev}/ .`; any later artifact can copy selected results from any preceding builder | References are backward-only, so no cycles or forward-declared graph; there is no Docker layer inheritance. |
| [BuildKit secret/SSH mounts](https://docs.docker.com/build/building/secrets/) | ❓ private dependencies—Nix has netrc/access tokens | ❓ The non-leaking Cixfile and remote-builder story is unspecified. |
| [reproducible builds](https://docs.docker.com/build/ci/github-actions/reproducible-builds/) | 🔁 Cixfile locks inputs and fixed-output fetches; `--cold` performs a clean RUN rebuild and audits both traced inputs and selected outputs. The CIP-87 hermetic fixture proves byte-identical convergence, while the deliberately warm-path-dependent proj1 fixture proves read-set divergence is rejected. | Product-wide sampled rebuild scheduling and refusal of every possible impure Nix escape hatch remain absent. |
| [Dockerfile here-documents](https://docs.docker.com/reference/dockerfile/#here-documents) | 🔁 Cixfile supports interpolated `FILE <<EOF` and builder-shell `RUN <<EOF`; scripts are real files copied into artifacts (D51/D55) | Docker also permits heredoc content directly on COPY and arbitrary RUN interpreters. |
| [`STOPSIGNAL`](https://docs.docker.com/reference/dockerfile/#stopsignal) | ⏳ future manifest field (`KillSignal=`), with stop timeouts | — |
| [`SHELL`, `ONBUILD`, `MAINTAINER`, parser directives](https://docs.docker.com/reference/dockerfile/#parser-directives) | ❌ no global shell directive, image inheritance, downstream triggers, or parser magic; copied scripts name their shell explicitly in `START` (D55) | Per-image default shell choice, inherited downstream triggers, frontend versioning, and direct compatibility with those Dockerfiles. |
| [`RUN --mount=cache/bind/tmpfs`, `--network`, `--security`](https://docs.docker.com/reference/dockerfile/#run) | 🔁 D71's builder underlay preserves the whole prior workspace outside store items and memo keys; `RUN` remains networkless and `FETCH` is the network boundary | No per-step bind/tmpfs/secret/security flags or per-step cache selection; there is deliberately no `CACHE` directive. |
| [Buildx / Bake / builder management](https://docs.docker.com/reference/cli/docker/buildx/) | ❌ Nix is the builder; remote/multi-platform 🔁 Nix distributed builds. As with Bazel, composix is the cage, not the compiler-orchestrator: ecosystem tools retain fine-grained incrementality in declared caches, while the sandbox supplies correctness without a hand-maintained dependency graph. | The Docker CLI's named builders, Bake graph, standard exporters, driver choices, and existing CI actions. |
| [`docker init`](https://docs.docker.com/reference/cli/docker/init/) | ❓ no generator exists | ❓ No generator for a Cixfile, manifest, Nix wrapper, or compose migration skeleton. |
| [build attestations](https://docs.docker.com/build/metadata/attestations/) | ❓ `drvPath` is provenance metadata | ❓ No standard SBOM/provenance attestation emission or policy story. |

## 4. Storage (coarse)

| docker | disposition | still missing |
| --- | --- | --- |
| [named volumes (including service sharing)](https://docs.docker.com/engine/storage/volumes/) | ✅ compose-local `shared:` surfaces are hermetic across declared `STATEDIR`/`DIR` members, with a stable group, setgid mode, and `UMask=0002`. | No generic Docker volume object, copy-up/`volume-subpath`, backup/restore, quota, snapshots, encryption, or cross-composite volume catalog. |
| [bind mounts (host paths in)](https://docs.docker.com/engine/storage/bind-mounts/) | ✅ `host:` binds pre-existing paths only, uses role-derived read-only/write behavior, requires a declared static `identity`, and never creates or chowns operator paths. `idmap: true` is the explicit acknowledgement for identity mapping. | No SELinux relabel flags, propagation modes, or arbitrary Docker mount syntax. |
| [tmpfs mounts](https://docs.docker.com/engine/storage/tmpfs/) | 🔶 `PrivateTmp` plus sized private `/dev/shm` through `SHM`; arbitrary tmpfs destinations remain unbuilt. | Arbitrary destination, modes, and swap policy. |
| [volume drivers / plugins](https://docs.docker.com/engine/extend/plugins_volume/) | ❌ filesystems are the host's business | Portable Compose declarations for remote, encrypted, cloud, clustered, and vendor storage backends; a driver interface. |
| [`volume prune`](https://docs.docker.com/reference/cli/docker/volume/prune/) / [`volume update`](https://docs.docker.com/reference/cli/docker/volume/update/) | 🔁 `cix clean --what=cache` removes expendable cache, logs are explicit opt-in, and state/DIR/shared are refused; `cix down --purge` confirms exact cix-owned private/shared paths. | Nix GC never cleans mutable role data; no generic volume mutation API. |
| [`volume create` / `inspect` / `ls` / `rm`](https://docs.docker.com/reference/cli/docker/volume/) | 🔁 private roots are unit-scoped and `shared:` is compose-local rather than a global object. | There is deliberately no standalone Docker-compatible volume catalog. |

## 5. Networking (coarse — the biggest *conscious* gap)

| docker | disposition | still missing |
| --- | --- | --- |
| [port publish / NAT](https://docs.docker.com/engine/network/port-publishing/) | 🔁 D24 makes each declared port a kernel-enforced bind capability; D29 listeners bind `-p name=addr` through transient socket units (`crates/cix-run/tests/system_projection.rs`) | No address translation, port inventory/collision manager, or per-service network namespace; ordinary declared ports have no policy-level bind-address control. |
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
`rollback` operate the system manager and require root. Netns, scale, secrets, limits,
and a reconciler remain deferred.

### Scheduled apps

An `APP` member can be scheduled at deployment time. `schedule` is raw systemd
[`OnCalendar=`](https://www.freedesktop.org/software/systemd/man/latest/systemd.time.html)
syntax; write and test it with `systemd-analyze calendar`, rather than translating cron.
`persistent` and `jitter` are optional direct spellings of `Persistent=` and
`RandomizedDelaySec=`. Omit either field to retain systemd's own default — cix supplies none.

```json
{
  "item": "renovate:stable",
  "schedule": "Mon *-*-* 03:00:00",
  "persistent": true,
  "jitter": "30m"
}
```

`schedule` is a hard error on a long-running `SERVICE`; scheduled app units are triggered by
their paired `cix-<compose>-<member>.timer`, while the composite target wants only that timer.
`cix up` activates it and `cix down` stops it. Inspect the next and last runs with
`systemctl list-timers 'cix-<compose>-*'`. The unary spelling is
`cix run <item> --schedule '<OnCalendar>'`; it creates the same systemd-native transient timer
shape for an APP.

### Observability and retention

Every generated service stamps indexed journald fields: compose members carry
`CIX_COMPOSITE`, `CIX_SERVICE`, and `CIX_ITEM`; transient runs carry `CIX_RUN` and
`CIX_ITEM`. `cix ps` reports the native systemd result, while `cix inspect --runtime` also
reports its invocation ID and exit status. Systemd spawn failures 200–245 are diagnosed by their
sandbox setup step; `watchdog` reads as “liveness watchdog missed”.

Set the compose-level field below when the composite needs a separate journald retention policy:

```json
{
  "logNamespace": true,
  "services": { "api": { "item": "api:v1" } }
}
```

This adds `LogNamespace=cix-<compose>` to every member. It is deliberately compose-only: the
operational consequence is real—read those records with `journalctl --namespace=cix-<compose>`
and configure that namespace's own journald retention/size policy. `cix logs` detects the field
and supplies the namespace automatically. Journald remains the logging driver and forwarding
owner; cix has no logging-driver fields.

| docker | disposition | still missing |
| --- | --- | --- |
| [Compose services](https://docs.docker.com/reference/compose-file/services/) | ✅ strict machine-format `compose.json` services resolve local tags/store paths and apply declared env/listener bindings plus a loud `shm` override. `grants:` is schema-reserved for future explicit loosening and is rejected today, so compose cannot silently widen device access (`crates/cix-compose/src/{model,resolve,generation}.rs`; `examples/compose/stack/compose.json`). | Docker Compose YAML compatibility, multi-file merging/includes, `extends`, device grants, and broad Compose-field coverage. |
| host cron / scheduler sidecar | 🔁 an APP member's compose `schedule` is raw systemd `OnCalendar=` with optional `persistent`/`jitter`; `systemctl list-timers` is the observation surface | No cron expression translation, overlap-policy vocabulary beyond systemd's native coalescing, history limits, or suspend field. |
| [`depends_on` / ordering](https://docs.docker.com/reference/compose-file/services/#depends_on) | 🔁 v0 Unix `edges` require their setup unit and put the consumer `After=` the producer service while exposing only declared runtime paths; systemd start-job completion makes those structural consumers wait for producer readiness (`crates/cix-compose/src/generation.rs`; `nix/scenarios/health.nix`) | ❌ `condition: service_healthy` is deliberately rejected rather than creating a second health graph. Restart propagation and network discovery remain. |
| [scale / replicas](https://docs.docker.com/reference/cli/docker/compose/scale/) | ⏳ no replica field or template units in v0 | Scaling, stable replica identities, placement, and lifecycle semantics. |
| [`env_file` / secrets](https://docs.docker.com/reference/compose-file/services/#env_file) | 🔁 v0 has explicit non-secret string `env` overrides plus CIP-81 file-only compose `secrets: { name: { file\|encrypted } }` → `LoadCredential=` delivery; `SECRET name AS VAR_FILE` supports `_FILE` images | Docker-compatible env-file parsing and raw secret environment delivery (refused). |
| [resource limits](https://docs.docker.com/reference/compose-file/deploy/#resources) | ⏳ no compose limits yet; future operator policy maps to systemd slice/unit properties | CPU, memory, IO, PID, and reservation controls. |
| [project namespacing](https://docs.docker.com/compose/how-tos/project-name/) | ✅ the compose name produces `cix-<name>.slice`/`.target`, `cix-<name>-<service>` units, and a `cix-compose-<name>` Nix profile (`crates/cix-compose/src/{generation,runtime}.rs`) | Docker's project-name flags and compatibility naming rules. |
| [`up`](https://docs.docker.com/reference/cli/docker/compose/up/) / [`down`](https://docs.docker.com/reference/cli/docker/compose/down/) / rollback | ✅ `cix up` resolves, writes `cix.lock`, builds/activates a profile; `cix down` unlinks managed units while retaining it; `cix rollback` activates the preceding generation (`crates/cix-compose/src/runtime.rs`; `examples/compose/stack/demo.sh`) | Rootless activation, a long-running reconciler, and Docker-compatible lifecycle flags/observation commands. |
| [`docker compose config`](https://docs.docker.com/reference/cli/docker/compose/config/) | 🔁 `cix compose check` resolves and semantically validates; `cix compose diff` dry-builds and compares the prospective generation without activation (`crates/cix-compose/src/{cli,runtime}.rs`) | A merged/canonical Compose configuration emitter, multi-file inputs, and Docker's config command/options. |
| [`watch` (dev mode)](https://docs.docker.com/compose/how-tos/file-watch/) | ✅ `cix watch` warm-rebuilds changed local members and selectively restarts changed services; source sync is ❌ because it makes the live service diverge from its artifact | Docker's `sync`/`sync+restart` actions; framework hot reload belongs in `nix develop`. |
| [profiles](https://docs.docker.com/reference/compose-file/profiles/) | ⏳ no Docker Compose service-profile selection exists; the Nix profile used for rollback is an unrelated homonym | Service selection, dependency validation, and profile-aware locking. |
| [`configs` top-level element](https://docs.docker.com/reference/compose-file/configs/) | ⏳ no compose config object in v0 | Top-level config objects, `ConfigurationDirectory=` materialization, ownership/mode, and attachment semantics. |
| [networks, volumes, secrets, configs as reusable top-level objects](https://docs.docker.com/reference/compose-file/) | 🔶 v0 ships named Unix edges and compose-local `shared:` directory identity; credentials and generic top-level objects remain separate work. | Generic external objects, networks, configs, and CIP-81 credential delivery. |
| [`version` marker (obsolete)](https://docs.docker.com/reference/compose-file/version-and-name/#version-top-level-element-obsolete) | ❌ | — |

## 7. Daemon & platform

| docker | disposition | still missing |
| --- | --- | --- |
| [`dockerd` (the daemon)](https://docs.docker.com/reference/cli/dockerd/) | ❌ systemd is the runtime; `cix` is a CLI + later a small reconciler (D9) | One versioned engine API owning lifecycle, images, networks, volumes, events, metrics, and remote automation; a unifying platform surface. |
| [`docker context` / remote hosts](https://docs.docker.com/reference/cli/docker/context/) | ❓ ssh is the transport today; `cix --host` sugar maybe ⏳ | ❓ Remote contexts. |
| [events API](https://docs.docker.com/reference/cli/docker/system/events/) | 🔁 journald/systemd events | Typed events API. |
| [logging drivers](https://docs.docker.com/engine/logging/configure/) | ❌ journald; forwarding is journald's job. Compose `logNamespace: true` is an opt-in per-composite retention boundary, not a cix driver. | Per-workload selection and portable Compose configuration for JSON, syslog, Fluentd, GELF, cloud, and plugin drivers. |
| [storage drivers](https://docs.docker.com/engine/storage/drivers/select-storage-driver/) | ❌ the Nix store | Platform/filesystem-specific runtime-layer choices and compatibility with Docker's mutable container filesystems. |
| [plugins](https://docs.docker.com/engine/extend/) | ❌ no plugin interface | Third-party volume, network, authorization, logging, and other daemon extensions. |
| [rootless mode](https://docs.docker.com/engine/security/rootless/) | 🔁 `--user` degraded dev mode exists (D13); full rootless is not a goal | A materially complete rootless mode. |
| [Docker Desktop / GUI](https://docs.docker.com/desktop/) | ❌ no desktop product | Supported macOS/Windows development, managed VM/updates, file sharing, proxy/VPN handling, credential integration, GUI diagnostics, extensions, and optional Kubernetes. |
| [`system df` / `info` / `prune`](https://docs.docker.com/reference/cli/docker/system/) | 🔁 tags are GC roots and an anonymous run holds a unit-lifetime root; stopping it makes its untagged item collectible (D7, D63) | ❓ No unified disk-usage, capability/status, or safe unused-resource cleanup view. |
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
| [capabilities (`--cap-add`)](https://docs.docker.com/engine/containers/run/#runtime-privilege-and-linux-capabilities) | ✅ semantic claims only: declared ports compile to kernel-enforced `SocketBindAllow=`/`SocketBindDeny=`; outside closed-root mode <1024 also grants `NET_BIND_SERVICE`. Closed roots reject direct privileged binds because `PrivateUsers=` makes that capability ineffective in the host network namespace, and teach an unprivileged port or named `LISTENER`. Undeclared bind denial is live-tested in `crates/cix-run/tests/system_projection.rs`. | ❓ One implemented semantic claim is not coverage of real workloads that need other narrowly scoped capabilities. |
| [userns-remap](https://docs.docker.com/engine/security/userns-remap/) | 🔁 ordinary phase-1 runs retain `DynamicUser` + idmapped mounts without a user namespace; CIP-84 `--closed-root` additionally sets `PrivateUsers=yes` so its synthetic three-entry NSS view and filesystem root have a user-namespace boundary | Docker's daemon-wide subordinate-ID remapping model is not reproduced; phase 2 makes the closed-root boundary universal only after the audit gate. |
| [AppArmor](https://docs.docker.com/engine/security/apparmor/) / [SELinux](https://docs.docker.com/engine/storage/bind-mounts/#configure-the-selinux-label) | ❓ host policy, likely out of manifest scope | ❓ Define labeling/profile behavior for store items and managed writable directories. |
| [secrets](https://docs.docker.com/compose/how-tos/use-secrets/) | ✅ `SECRET` needs plus compose file/encrypted sources compile to per-unit `LoadCredential=`/`LoadCredentialEncrypted=`; salted-HMAC composite state restarts only consuming services on rotation | Cluster distribution, raw env delivery, and a secret-value CLI flag (refused). |
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
| [`docker pass`](https://docs.docker.com/reference/cli/docker/pass/) | 🔁 host credential files plus per-project FETCH consent and `cix credentials revoke` | No OS-keychain integration or generic secret-value manager. |

The [top-level Docker CLI reference](https://docs.docker.com/reference/cli/docker/) was used as
the thoroughness checklist. Concepts without an existing composix decision are marked ❓ rather
than assigned a disposition here.

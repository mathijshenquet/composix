# cix import prototype log

## 2026-07-28  — Track start

- Goal: test whether offline OCI/Docker image import into a Nix store item plus generated
  `cix-spec.json` is a cheap, useful migration bridge.
- Scope is limited to this crate, workspace/CLI wiring, and this append-only report. The normal
  `cix-run` projection semantics will not be changed for full-rootfs images.
- Read `docs/design.md` D8, D11, D20–D22 and `docs/docker.md` scope/section 1. The current runner
  implements cix-spec v1/v2, while D22's v3 sparse projection is a design decision not yet
  implemented. Imported full root filesystems therefore need a separate `RootDirectory=`
  experiment and are not expected to run through today's `cix run`.
- The ambient shell has not loaded direnv. Rust commands will run through `devenv shell` so the
  repository's pinned Rust toolchain is used.
- Planned prototype: accept a Docker archive tar or OCI layout directory; parse image config;
  safely apply ordered gzip/uncompressed tar layers with OCI whiteouts; write `rootfs/` and a
  cix-spec v2 file; invoke `nix store add-path`; cover archive parsing, whiteouts, and spec
  generation with fixture tests before using real nginx and redis images.
- Known question to measure: OCI image metadata gives exposed container ports but not the
  operator's host-port choice. The generated spec can faithfully use fixed-value ports, while
  collision/exposure remains an operator/compose concern.

## 2026-07-28 — Importer implementation slice

- Added offline readers for single-image Docker archives and single-manifest OCI layouts.
  Registry access and multi-platform selection are deliberately absent. OCI gzip and plain-tar
  layers work; zstd layers are detected and rejected with a precise error for now.
- Layer application scans each layer once for whiteouts, applies deletions/opaque-directory
  clearing to the lower rootfs, then extracts non-whiteout entries. Doing deletions before
  extraction matters: an opaque marker removes lower children, not files created by its own layer.
- Metadata mapping is implemented as cix-spec v2: `Entrypoint` + `Cmd`, string env defaults,
  fixed TCP/UDP port values, and `Volumes` copied to `dirs.state`. Bare entrypoint names are
  resolved through the image's `PATH` when present in the assembled rootfs.
- `WorkingDir` and `User` become warnings/findings because v2 has no representation. Arbitrary
  Docker volumes also become a finding: the importer records (for example) `/data` faithfully,
  but the v2 validator requires exactly `/var/lib/<name>`. Silently remapping it would change the
  image contract.
- Fixture tests cover Docker archive input, OCI layout + gzip input, ordinary and opaque
  whiteouts, config mapping, and traversal rejection. `cargo test -p cix-import` and
  `cargo clippy -p cix-import --all-targets -- -D warnings` pass under `devenv shell`.
- Next: verify the integrated CLI and then exercise real pinned nginx and redis images.

## 2026-07-28 — Real nginx import and run

- Pulled `docker.io/library/nginx:alpine` once with skopeo 1.23.0 into an OCI layout, selecting
  linux/amd64. Host skopeo had no containers trust-policy file, so this one-time public pull used
  its explicit `--insecure-policy`; registry transport is not part of the importer.
- Tested manifest digest:
  `sha256:1d40e3eb3bf4f138de1d67193f2aa5309fcaf343eb5ffadbf5e9439de1eb1ebb`;
  config digest:
  `sha256:f0ba77f796e57c6fa89ae7f4fdad1665d6fcbd8e3f211535120542b337f9959e`.
  This was nginx 1.31.3, entrypoint `/docker-entrypoint.sh`, command
  `nginx -g "daemon off;"`, exposed TCP 80, no volumes, `WorkingDir=/`.
- Offline `cix import <layout> --name nginx` produced
  `/nix/store/ib41fmr50npn8iwg5sshslv6wl53dh4z-cix-import-nginx` (65 MiB apparent size).
  Generated exec/env/port metadata matches the config; `WorkingDir=/` is the sole warning.
- A baseline `RootDirectory=<item>/rootfs` unit with every normal cix hardening control reached
  the real entrypoint and nginx, then failed on read-only `/var/cache/nginx/client_temp`.
- `CacheDirectory=cix-import-nginx:nginx` is not a solution under `RootDirectory` on this
  systemd 257 host: namespace setup fails with `status=226/NAMESPACE`, `File exists`, because
  the full rootfs already contains `/var/cache/nginx`. This is precisely the projection/idmap
  mismatch that D11/D22 avoid for native sparse items.
- The successful unit kept `DynamicUser`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`,
  `NoNewPrivileges`, `RestrictSUIDSGID`, all kernel/control-group protections,
  `LockPersonality`, `MemoryDenyWriteExecute`, `SystemCallFilter=@system-service`,
  `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`, and a bounding+ambient set containing only
  `CAP_NET_BIND_SERVICE`.
- Required writable surfaces were ephemeral tmpfs mounts over `/var/cache/nginx`, `/run`, and
  `/var/log/nginx` (all `rw,nodev,nosuid,mode=1777`). The log mount is needed because Docker's
  `/var/log/nginx/{access,error}.log -> /dev/std{out,err}` convention does not work with a
  systemd journal stream: nginx reopening the journal socket through those paths gets `ENXIO`.
  Consequence: nginx's own logs land in the ephemeral mount rather than journald.
- The entrypoint's attempted edit of `/etc/nginx/conf.d/default.conf` detected the immutable
  filesystem and continued. No template directory existed in this image, so no other `/etc`
  mutation was required. nginx warned that its `user nginx` directive is ignored when the master
  is already the non-root dynamic user; this was harmless.
- End-to-end result: `curl http://127.0.0.1/` returned the stock “Welcome to nginx!” page.
  `systemd-analyze security` rated the live unit 3.2 “OK”. The transient unit was stopped and
  verified inactive.
- Hardening dropped: none of the standard cix controls. Compatibility added three broad,
  image-path-specific writable tmpfs mounts; persistence and native journald log capture were
  lost. Production code would need a declared writable-path/overlay strategy instead of these
  experiment-only mounts.

## 2026-07-28 — Real redis import and run

- Pulled `docker.io/library/redis:alpine` once as linux/amd64 OCI. Tested manifest digest:
  `sha256:465aff338d817971674ff1ec3c0d59182e2b687018e87bf94b6e1491d0bb79e2`;
  config digest:
  `sha256:a597386e6d2ef50b536ae3c77aa77e8c3772631abe949882f9f2a7d1a3065b79`.
  This was Redis 8.8.1, bare entrypoint `docker-entrypoint.sh`, command `redis-server`, exposed
  TCP 6379, `WorkingDir=/data`, and (notably for this current tag) no declared `Volumes`.
- Offline import produced
  `/nix/store/sqk0prhcn4d01h5rdp7hgyd2yas96m4d-cix-import-redis` (112 MiB apparent size).
  PATH lookup correctly resolved the bare entrypoint to
  `rootfs/usr/local/bin/docker-entrypoint.sh`. `WorkingDir=/data` is the only metadata warning.
- Successful transient-unit additions were `WorkingDirectory=/data` and an ephemeral writable
  tmpfs over `/data` (`rw,nodev,nosuid,mode=1777`). The full standard hardening set survived,
  including `DynamicUser`, `MemoryDenyWriteExecute`, and `SystemCallFilter=@system-service`;
  both ambient and bounding capability sets were empty.
- The entrypoint saw that it was already non-root and skipped its root-only `chown`/`setpriv`
  path. Redis loaded its bundled modules, listened on 6379, and returned the RESP frame
  `+PONG\r` to a raw protocol PING. `systemd-analyze security` rated the live unit 3.1 “OK”.
  The transient unit was stopped and verified inactive.
- Redis warned that host `vm.overcommit_memory` is disabled. `ProtectKernelTunables=yes` remained
  in force; changing host sysctls is correctly an operator concern, but production Redis would
  need the host prerequisite documented/checked.
- Redis also bound all host interfaces without authentication, exposing the already-known
  compose/networking gap: declaring port 6379 grants network access but does not provide Docker's
  port publication or isolation semantics.
- Hardening dropped: none. Compatibility/persistence compromise: `/data` is an image-specific
  writable path outside D11's managed `/var/lib/<name>` shape, so this run used tmpfs and loses
  data at stop. A persistent bind would either lose the dynamic-user idmap on systemd 257 or
  require a semantic path remap/command override, neither of which is available in today's spec.

## 2026-07-28 — Format and determinism spot checks

- Converted the already-downloaded nginx OCI layout to a real Docker archive with skopeo, then
  imported that tarball. It produced the exact same store path as the OCI-directory import.
- Re-importing the OCI layout also produced that same path. For this fixture, layer application,
  generated JSON ordering, and `nix store add-path --name cix-import-nginx` are deterministic.
- Caveats: no layer digest verification is implemented; timestamps and ownership are discarded
  by Nix's NAR/store model; xattrs (including file capabilities), devices, and setuid semantics
  are not preserved. Those losses improve some safety properties but mean the result cannot be
  claimed byte/behavior-equivalent for arbitrary OCI images. Multi-image Docker archives,
  nested/multi-platform indexes, and zstd layers are explicitly rejected.

## 2026-07-28 — Safety pass and full verification

- Added a regression test and guard for whiteouts whose parent traverses a symlink installed by a
  lower layer. Without the guard, a malicious image could redirect a deletion outside the
  assembly root. The outside fixture now remains untouched and import fails closed.
- The OCI fixture's generated `cix-spec.json` is now loaded through the real `cix-run` parser in
  tests, confirming the emitted v2 shape is accepted when metadata is representable.
- Final verification under the pinned devenv toolchain:
  `cargo fmt --all -- --check`, `cargo test -p cix-import`,
  `cargo clippy -p cix-import --all-targets -- -D warnings`, `cargo test --workspace`, and
  `cargo clippy --workspace --all-targets -- -D warnings` all pass. The new crate has four tests;
  the full workspace suite has no failures.

## 2026-07-28 — Final report and verdict

### Executive answer

The mechanical import is cheap; honest runtime compatibility is not. This prototype imports both
requested offline formats into stable Nix store items, maps the straightforward config fields,
and ran real nginx and Redis images under strong systemd hardening. It also exposed that the
useful part of the promise is not “untar into the store”: it is a second, full-rootfs runtime
model with mutable-path inference, OCI identity semantics, logging adaptation, and a long
compatibility tail.

**Verdict: distraction** as a product track in the proposed `cix import` form. Do not merge this
prototype subcommand. A later read-only `cix migrate`/inspection tool that extracts metadata and
generates a native Cixfile skeleton could reuse the parsing work without promising OCI runtime
compatibility.

### What works

- A single-image `docker save` archive and a single-manifest OCI layout directory import fully
  offline. There is no registry client.
- Ordered uncompressed/gzip layer application handles ordinary and opaque OCI whiteouts.
- The output is one Nix store item containing `rootfs/` plus deterministic pretty-printed
  `cix-spec.json`. Repeated OCI imports and a Docker-archive conversion of the same nginx image
  produced the identical store path.
- Config mapping works for the tested shapes: `Entrypoint + Cmd`, env defaults, numeric TCP/UDP
  `ExposedPorts` as fixed-value ports, and `Volumes` as state-dir declarations. Bare entrypoints
  can be resolved through the image PATH.
- nginx served its stock page and Redis returned PONG from their real entrypoints. Both retained
  all normal cix hardening controls. nginx needed only the semantically implied low-port
  capability; Redis had an empty capability set.
- The expanded-cost evidence is nontrivial: the compressed OCI layouts were about 25 MiB nginx /
  36 MiB Redis, while the resulting full-rootfs store items were about 65 MiB / 112 MiB. Nix can
  distribute each item reliably, but it gets no cross-image layer reuse at this item granularity.

### What breaks or remains prototype-grade

- Today's `cix run` cannot run the generated spec correctly. It resolves
  `rootfs/<entrypoint>` in the store but does not enter that rootfs, so absolute paths used by the
  entrypoint resolve against the host namespace. A dedicated RootDirectory service kind would be
  required. D22's sparse projection deliberately does not describe a full rootfs.
- `WorkingDir` and image `User` have no spec representation. Arbitrary Docker volumes such as
  `/data` cannot satisfy v2's one-component-under-`/var/lib` state-dir rule. The prototype warns,
  but may therefore emit a spec that the current parser rejects when such a volume is present.
- Writable needs are not described completely by image metadata. nginx needed cache, run, and log
  paths that were not `Volumes`; Redis needed its `/data` working directory writable despite the
  tested current image declaring no volume. Discovering these by failure is not a product model.
- systemd 257 managed-directory aliases fail namespace setup when `RootDirectory` already
  contains the destination (`File exists`). Broad tmpfs overlays made the demos run, but they
  discard state. A persistent bind to arbitrary image paths loses the DynamicUser idmap that
  motivated D11's narrowed native model.
- Docker's log symlinks to `/dev/stdout` and `/dev/stderr` do not translate to journald sockets:
  nginx gets `ENXIO` when reopening them. The workaround makes logs ephemeral rather than
  journal-native.
- The importer does not verify descriptor/layer digests, retain provenance/config metadata,
  support zstd, choose from multi-platform indexes, select among multi-image archives, or
  preserve xattrs/file capabilities, ownership, device nodes, and setuid semantics. The last
  group also cannot be faithfully represented by a normal Nix store path.
- The imported item is initially unrooted and can be garbage-collected unless the user tags or
  otherwise roots it. Error recovery, progress output, disk-space checks, and interrupted-import
  behavior are prototype-level.
- Entrypoint arguments containing shell `$` syntax collide with cix's own exec interpolation
  model. Healthchecks, stop signals, labels, annotations, and other OCI config fields are ignored.

### Irreducible runtime differences

- **UID assumptions:** image `/etc/passwd` identities are not DynamicUser. Store ownership
  normalizes to root and NAR does not carry Docker layer ownership semantics. nginx's `user nginx`
  directive was ignored because the master was already a dynamic non-root user; Redis skipped its
  root-only `chown`/`setpriv` path. Other images may require a fixed numeric UID, root startup, or
  ownership changes. Supporting those weakens the current identity/persistence model rather than
  merely adding a parser field.
- **Writable-root assumptions:** OCI images assume an overlay root where any path may become
  writable and changes disappear with the container unless mounted. Composix assumes an immutable
  item plus declared, idmapped writable capabilities. Automatically overlaying the entire imported
  root would recreate container semantics, hide undeclared writes, complicate upgrades, and make
  persistence accidental.
- **Mutating entrypoints:** nginx's `/etc` edit was defensive and skipped a read-only file.
  Entrypoints that run `apt`, `apk`, write certificates/config anywhere under `/etc` or `/usr`, or
  install plugins at startup will fail. Granting them a writable root turns the “import bridge”
  into an OCI runtime with mutable package state—the compatibility surface explicitly outside
  composix's thesis.
- **Network/operator semantics:** exposing a port in image metadata says what the process listens
  on, not whether/how the operator publishes it. Redis consequently bound the host network on all
  interfaces and warned about missing authentication. Docker bridge/NAT/isolation cannot be
  inferred or replaced by the generated fixed port.
- **Host/kernel expectations:** Redis's overcommit warning is a real host prerequisite. Some
  images require sysctls, devices, capabilities, LSM labels, or seccomp exceptions that cannot be
  safely inferred. Keeping `ProtectKernelTunables` was correct; compatibility cannot silently
  authorize host changes.

### Effort estimate

- A narrow, explicitly curated beta would take roughly **6–10 engineer-weeks**: digest and
  compression/platform correctness; adversarial extraction/conformance work; a new full-rootfs
  runtime type; cwd/user/log handling; declared persistent overlays with safe DynamicUser
  ownership; provenance/GC UX; and an integration corpus.
- A credible “import arbitrary Docker images” claim is at least **3–6 months plus ongoing
  compatibility maintenance**, because writable paths, UID transitions, entrypoint mutations,
  capabilities, and host prerequisites are workload-specific. That estimate still excludes
  registry auth and Docker networking parity.
- The parser/unpacker itself is perhaps 1–2 weeks of production hardening. It is not the dominant
  cost and does not, by itself, soften the migration cliff: users need the imported service to run
  with persistence, logging, and isolation they can trust.

### Merge assessment and cleanup

The code is small and tested, but the command is not clean enough to merge even with an
“experimental” label: its output suggests compatibility that current `cix run` cannot provide,
and representable-looking Docker volumes can create an invalid v2 spec. Keeping it would also
create pressure to evolve a second runtime model before the native D22/compose model is complete.
The branch/report should remain the experiment's value.

All named transient experiment units were stopped/reset; no `cix-import-*` units and no listeners
on ports 80 or 6379 remained. The temporary OCI layouts and Docker archive under
`/tmp/cix-ocimport.FSKObs` were permanently removed after the recorded digest/size checks. The two
unrooted Nix store outputs remain subject to normal garbage collection.

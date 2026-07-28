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

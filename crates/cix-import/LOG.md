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

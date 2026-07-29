# track/buildtool — single-stage `BUILD rust` + examples/build/projB (D37 c, increment 1)

Read AGENTS.md first. Authoritative design: docs/design.md D37 (c) and docs/cixfile-build.md
"Variant A" — but ONLY the tooling-integration half. Explicitly OUT of scope this track:
`STAGE`, `COPY --from`, `OUTPUT`, `DEFAULT`, `BUILD pnpm` (increment 2, separate track).
D37/cixfile-build.md win on conflict.

## Surface (this increment)

One new directive in the Cixfile grammar:

    BUILD rust <source-dir>

- Single-output form only: the crate at `<source-dir>` (sibling path, like COPY sources) is
  built and its binaries land in the item at a fixed, documented location; `EXEC`/`PATH` can
  reference them. If the crate has multiple binaries, require the service to name one
  explicitly via its EXEC path — do not invent a selector flag this increment.
- Fixed Variant A crane semantics, non-negotiable (cixfile-build.md): `Cargo.lock` REQUIRED
  at the source root (absence = line-numbered parse/build error, never an ambient
  toolchain); `rust-toolchain.toml` required likewise per cixfile-build.md; locked vendoring
  automatic and offline; internally a deps-then-bin split for caching is fine but NOTHING
  stage-like surfaces in the Cixfile.
- Lock story: whatever input pinning `BUILD rust` needs beyond Cargo.lock goes in
  `Cixfile.lock` per existing per-input conventions (D32); document exactly what is hashed.
  "Determinism is more than having a lockfile" — the full input surface must be pinned.
- Codegen (decided with Mathijs, 2026-07-29): **call crane, don't reimplement it.** Crane is
  a cix-owned pinned tool input: cix knows a fixed crane rev, pins it (rev + narHash) in
  `Cixfile.lock` automatically next to nixpkgs (the D32 "cix-owned constant" pattern — the
  user never writes a crane FROM), and the generated non-flake nix does
  `craneLib = (import (fetchTarball {...narHash...})).mkLib pkgs`, then
  `cleanCargoSource` → `buildDepsOnly` → `buildPackage`. No ad-hoc flakes, no vendored
  crane copy (both considered, rejected for now).
- Toolchain policy (increment 1): build with the locked nixpkgs rustc. `rust-toolchain.toml`
  is required, READ, and enforced as a *compatibility gate*: hard, line-numbered error when
  it demands a channel/version the locked nixpkgs rustc does not satisfy — no ambient
  toolchain, no silent substitution. Full honoring via a pinned rust-overlay is a separate
  later decision; say so in the docs.
- Error legibility minimum: `cix build` prefixes crane/nix build failures with the Cixfile
  source span of the responsible BUILD line.

## examples/build/projB (the proof)

A deliberately plain single-binary rust service: small axum/std HTTP responder or
equivalent, Cixfile of the form FROM…AS pkgs / BUILD rust ./app / SERVICE with EXEC of the
built binary + one declared port. No multi-stage, no frontend. This example is the
acceptance test of the increment: `cix build` it, `cix run` it, curl it.

## Verification gate

1. Workspace fmt/build/clippy -D warnings/tests clean; new parser/codegen unit tests incl.
   the missing-Cargo.lock and missing-toolchain error paths (line-numbered, quoted source).
2. Determinism: build projB twice (fresh temp lock state second time per lock conventions)
   → identical store path; record both paths in the LOG.
3. Live (sudo allowed): `cix build examples/build/projB -t projb && cix run projb` →
   HTTP response received; stop, clean up.
4. Tour: add a build scenario page for projB if deterministic under the normalizers (store
   hashes normalize already); otherwise explain in the LOG. Drift green either way.
5. `nix build .#checks.x86_64-linux.vm-dogfood` still passes.
6. docs: cixfile.md gains the BUILD rust section (this increment's surface only, honest
   about what is deferred to increment 2); docker.md Building-section rows updated where
   they claim RUN/build gaps this narrows — cite D37.
7. Commit on track/buildtool. No commit = failed task.

## Log

Keep .dev/specs/track-buildtool.LOG.md current (append-only, timestamped; transcripts and
the determinism store paths).

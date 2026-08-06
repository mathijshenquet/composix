# build-args-tag — the declared `TAG` line

Status: **deferred** (2026-08-06, Mathijs at CIP-113 adoption: "haal
TAG er nog maar even uit, doen we tzt" — the design below is the
worked v4 pass, parked intact for a later round; CIP-113 shipped
without it, tagging stays CLI-side for now).

Extracted verbatim from build-args v4:

The v3 sketch (CLI interpolation, `--tag 'app:${VERSION}'`) kept
tagging freehand — the Docker shape, and the janky part. The v4 cut
inverts it: **the file declares its own identity.**

Docker is the outlier here, not the norm. Everywhere else the
artifact's name lives in source: Cargo.toml's name+version, Maven
coordinates, package.json, nix flake output names — and Docker's own
ecosystem reinvented it the moment builds got matrices:
`docker buildx bake` declares `tags = ["app:${VERSION}"]` in
source-side HCL. The freehand `-t` survives only where there is no
file to declare it in.

Proposal:

- **ONE `TAG <ref>` line per Cixfile** (Mathijs, 2026-08-06: uniform —
  the file has one identity, the tag-per-Cixfile semantics we already
  carried). It interpolates LET/ARG: `TAG app:${VERSION}`. No alias
  TAG lines — aliases (`app:latest`) are index-level moves via
  `cix tag`, after the build, where retagging already lives. Today's
  tag surface is CLI-only (`BuildOptions.tag` → the registry's
  `tag_artifact`); TAG feeds that same seam from source.
- **`cix build` applies the declared tag by default**; `--all-args`
  yields tag-per-cell automatically because the one template resolves
  per cell — the CI matrix story becomes declaration, not flag
  choreography. `--tag` stays as an explicit override move (the
  `--override-input` shape: visible, never ambient).
- **Collision guard**: under `--all-args`, a TAG template that does
  not mention any ARG resolves identically for every cell — that is
  an error (declare the interpolation or build one cell), never a
  silent last-writer-wins.
- **Identity is not content**: TAG lines should not participate in
  build keying/sourceHash — retagging must not rebuild. Needs an
  explicit carve-out in the fingerprint, and is consistent with the
  manifest recording selection (the artifact knows which cell it is;
  the tag names it outward).

Open at deferral: TAG placement (prelude vs APP block; prelude
proposed) and interaction with index namespaces/qualified refs.

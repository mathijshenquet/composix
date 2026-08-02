# emit-nix — cix build as the *2nix that ends *2nix

Status: **draft** (2026-08-02, from Mathijs's prompt: prospects of cix
build inside the nix world purely as DX — no translator tools — with
fantasy allowed and an adversarial eye required; §5).

## 1. The problem

Packaging software for nix means teaching the sandbox about network
dependencies. Every language ecosystem got its own translator
(crate2nix, node2nix, poetry2nix, gomod2nix, …) or a mega-FOD hash
(`cargoHash`, `npmDepsHash`) with the fake-hash-twice dance. The
translators *guess* from ecosystem metadata what a build will fetch.
cix's FETCH machinery *observes* what it actually fetched —
TOFU-pinned, consumed-set-narrowed, per-path content-hashed in
`Cixfile.lock`, offline-replayable (`--cold`). Observation is strictly
more general than translation: it covers ecosystems without lockfiles,
tools that fetch mid-build, and polyglot builds with one mechanism.

## 2. Today, concretely

**Current nix definition** (the status quo being replaced) —
gitsitter via `buildRustPackage`:

```nix
rustPlatform.buildRustPackage rec {
  pname = "gitsitter";
  version = "0.2.1";
  src = fetchFromGitHub {
    owner = "mathijshenquet"; repo = "gitsitter";
    rev = "29c8a2d…"; hash = "sha256-AAAA…";      # dance 1
  };
  cargoHash = "sha256-BBBB…";                     # dance 2: build,
  buildInputs = [ openssl libgit2 sqlite ];       # fail, paste real
  nativeBuildInputs = [ pkg-config ];             # hash, build again
  GIT_COMMIT_HASH = src.rev;
}
```

One opaque `cargoHash` covers every crate; a polyglot app repeats this
per ecosystem with a different tool each time.

**The same build as a Cixfile** (works today, 14 lines, no hash dance
— the lock is written by observation):

```dockerfile
FROM github:NixOS/nixpkgs/9cf7092… AS pkgs
FROM github:mathijshenquet/gitsitter AS src

BUILDER build
  IMPORT ${pkgs.cargo} ${pkgs.rustc} ${pkgs.pkg-config} …
  ENV GIT_COMMIT_HASH = ${src.rev}
  COPY ${src}/ .
  FETCH cargo vendor --locked vendor > .cargo/config.toml
  RUN cargo build --release --locked --offline

ITEM gitsitter
  COPY ${build}/target/release/gitsitter /bin/gitsitter
```

## 3. The bridge, in three tiers

**Tier 1 — `cix-lib.buildCixfile` (flake library, no generated files,
no IFD).** The key insight: `Cixfile.lock` is committed JSON carrying
per-fetch consumed-path hashes — pure eval can construct the
derivation graph FROM DATA, exactly how `importNpmLock` reads
package-lock.json. Fantasy syntax:

```nix
# flake.nix of any project
inputs.cix-lib.url = "github:mathijshenquet/composix?dir=nix/lib";
outputs = { self, nixpkgs, cix-lib, … }: {
  packages.x86_64-linux.gitsitter = cix-lib.buildCixfile {
    src = ./.;              # Cixfile + Cixfile.lock live here
    item = "gitsitter";
  };
};
```

Under the hood, per lock entry: each FETCH pin becomes a fixed-output
derivation re-running the recorded command with network, output =
exactly the consumed set, `outputHash` from the lock; the RUN becomes
a normal sandboxed derivation over (sources, FODs, the versioned cix
skeleton). Emitted semantics are the COLD path by definition — the
underlay/warm world stays cix-side.

**Tier 1b — the in-nixpkgs form** (assume cix itself is packaged in
nixpkgs). `buildNpmPackage`/`importCargoLock`-shaped in-tree builder:

```nix
# pkgs/by-name/gi/gitsitter/package.nix
{ lib, buildCixPackage, fetchFromGitHub }:
buildCixPackage rec {
  pname = "gitsitter"; version = "0.2.1";
  src = fetchFromGitHub { owner = "mathijshenquet"; repo = "gitsitter";
                          tag = "v${version}"; hash = "sha256-…"; };
  cixLock = "${src}/Cixfile.lock";   # upstream ships its own lock
  item = "gitsitter";
  meta.license = lib.licenses.mit;
}
```

Two structural upgrades over tier 1:

- **The lock can live UPSTREAM.** `importCargoLock` already builds
  per-crate FODs in pure eval from the packaged project's own
  Cargo.lock; `buildCixPackage` is that, generalized to every
  ecosystem at once. Packaging collapses to "point at the repo" —
  six lines, no vendored metadata in nixpkgs, because the project
  carries its own machine-verifiable build+fetch description with
  per-path pins. (No IFD anywhere: the lock is a committed file in
  `src`, eval reads data, never build outputs.)
- **Adversarial turn 3 dissolves.** With cix in nixpkgs there is no
  emitter mirroring the sandbox: the builder runs `cix build --cold`
  INSIDE the derivation (cold never fetches, D69e — the FODs supply
  the FETCH snapshots, cix replays offline). One implementation, no
  byte-identity shadow to maintain. The residual engineering question
  moves: cix's inner sandbox (bubblewrap/userns) must run nested
  inside the nix build sandbox, or grow a mode that trusts the outer
  nix sandbox for isolation and only arranges the filesystem view —
  that mode is the honest open item of this tier.

**The pure-build cut** (what composix-without-compose-and-pack IS,
precisely — the seam tier 1b exposes): the D41/D68 line turns out to
be the product boundary. `BUILDER` blocks + `ITEM` outputs are the
complete build product — manifest-less store trees, which is exactly
what a nixpkgs package is. `SERVICE`/`APP` = the same build product
PLUS a runtime contract carried as data in the tree. So the cut is
not a fork but a projection:

- **What nixpkgs consumes**: Cixfile parser + builder engine + lock +
  `--cold` replay + ITEM assembly. No index (nixpkgs IS the
  distribution), no unit generation, no compose — the entire runtime
  half is absent, not stubbed. Concretely a crate seam:
  cix-build + the cixfile parser, with the manifest/spec dependency
  feature-gated to ITEM-only.
- **What rides along for free**: a SERVICE built through the same
  builder still emits its manifest as inert data — which is the
  D68+D65 composed-ITEM route: a NixOS module could later consume
  that manifest and generate units nixpkgs-side. The runtime thesis
  re-enters as an optional consumer of data, never as a build
  dependency.
- **The strategic read**: the build half is independently valuable to
  people who will never run a cix service — an adoption funnel
  (meet cix as "the *2nix killer", discover the runtime later), at
  the cost of maintaining a public builder seam for non-cix
  consumers. The thesis stays intact because the coupling was always
  one-directional: closed-root needs closure-complete builds; builds
  never needed the runtime.

**Tier 2 — `cix build --emit-nix <dir>` (generated standalone nix,
cix-free at build time).** Same graph, written out as boring committed
`.nix` — the form nixpkgs-upstream could take today (IFD is banned
there; committed-generated is the Cargo.nix-shaped precedent, and an
in-tree `buildCixPackage` would be `buildNpmPackage`-shaped). Sketch
of the generated file:

```nix
# generated by cix build --emit-nix — do not edit
{ pkgs }: let
  fetch_build_3 = pkgs.stdenvNoCC.mkDerivation {
    name = "cix-fetch-build-3";
    outputHashMode = "recursive"; outputHashAlgo = "sha256";
    outputHash = "sha256-NywkGX47…";     # lock: fetches["builder:build:3"]
    nativeBuildInputs = [ pkgs.cargo ];
    buildPhase = "…the FETCH command…; keep only consumed paths";
  };
in pkgs.stdenvNoCC.mkDerivation {
  name = "cix-item-gitsitter";
  buildPhase = "…the RUN command inside the cix skeleton…";
  … }
```

**Tier 3 — fantasy horizon.** `cix vendor -- <any command>`: no
Cixfile at all — observe an arbitrary command's fetches, emit lock +
nix (the universal escape for gradle/bazel-shaped builds). And when
dynamic derivations (RFC 92) stabilize, tier 2's commit step
dissolves: the lock IS the drv generator.

## 4. The flow, now vs then

**Now** (polyglot app, Rust + npm): pick two tools; run/maintain both
translations; two fake-hash dances; every dependency bump = regenerate
+ two opaque hash flips in the PR; reviewers see `sha256-B…` →
`sha256-C…` and approve blind.

**Then**: write the Cixfile (or `cix vendor`); `cix build` online once
— the lock records every fetched path with its own hash; commit
Cixfile + lock; `nix build` anywhere via tier 1, no cix installed for
consumers; dependency bump = `--update-lock`, and the PR shows a
per-dependency lock diff — reviewable provenance instead of one
opaque hash. That diffability is quietly the strongest pitch: a
`vendorHash` flip is unreviewable; a lock diff names what changed.

## 5. Adversarial turns

1. **"Nixpkgs already rejected granularity"** — the ecosystem migrated
   crate2nix→`cargoHash` partly BECAUSE generated-file churn drowned
   review. Concession: real, which is why tier 1 (eval-from-lock, zero
   generated files) is the primary form and tier 2 exists only for the
   upstream boundary. The churn argument dies when nothing is
   generated.
2. **"TOFU is not review."** First lock creation trusts the network; a
   registry compromised at lock time is a pinned compromise.
   Counter: `cargoHash` has the identical trust shape (you pin what
   you happened to vendor). Stronger mitigation available to us and
   not to mega-FODs: **cross-check observed content against the
   ecosystem's own checksums where they exist** (Cargo.lock carries
   per-crate sha256s — verify observation against declaration, refuse
   on mismatch). Observation + declared-checksum cross-check beats
   either alone.
3. **"The emitter is a second implementation of your sandbox."** The
   sharpest turn. Emitted builds must byte-match `cix build --cold` or
   hashes silently diverge, and now every skeleton/sandbox change has
   a shadow copy in the emitter. Survives only with: one shared,
   versioned definition of the skeleton that both consume, and a CI
   acceptance test pinning byte-identity emitted-vs-cold for the
   compare fixture. If that test is too expensive to keep, this CIP
   should be rejected — drift here is silent corruption.
4. **"Network drift breaks FOD rebuilds."** Upstream deletes a crate →
   the FOD fails on any cache-miss rebuild. Concession: true, and
   identical to every fetcher in nixpkgs today; caches and cix's own
   snapshot replay are the standard answer. No new exposure, no new
   protection.
5. **"Maintainers must install cix to update."** True at lock time —
   same as npm for package-lock.json. For flakes this is normal; for
   nixpkgs upstream it is the real adoption barrier and gates on cix
   itself being packaged there first. v1 therefore claims flakes-only;
   upstream is a horizon, not a promise.
6. **"Cross-compilation and meta."** nixpkgs cares about cross,
   licenses, platforms; cix has no cross story and manifests carry no
   meta.license. Concession: tier 2 output is not nixpkgs-grade until
   both exist; flake users can splice meta manually.

## 6. Recommendation

v1 = tier 1 (`cix-lib.buildCixfile`, eval-from-lock) + the byte-
identity acceptance test against `--cold`, plus the Cargo.lock
checksum cross-check from turn 2. Tier 2 (`--emit-nix`) lands behind
the same test. `cix vendor`, dynamic-drvs, and nixpkgs upstream stay
recorded horizons. ROI check before building: the tracefast track's
subprocess/cost table should confirm where the DX win concentrates.

## 7. Open questions

1. Does tier 1 live in this repo (`?dir=nix/lib`) or a sibling flake?
2. `cix vendor -- <cmd>` in v1 scope or horizon?
3. Is the checksum cross-check (turn 2) a FETCH feature independent of
   this CIP? (It hardens plain cix too — arguably split it out.)

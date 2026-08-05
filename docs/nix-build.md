# Building gitsitter three ways

This is a measured comparison of three ways to build the same Rust program:
gitsitter's upstream flake, an idiomatic crane flake, and a Cixfile. It is not
a claim that one route is generally faster or “more Nix”. The measurements
below are one dated run on one machine, and the awkward results are part of
the comparison.

The source is gitsitter commit
`29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd`. The local crane and Cix routes
also pin nixpkgs at `9cf7092bdd603554bd8b63c216e8943cf9b12512`.
The crane input resolved to `fe4c37e1c9e4d5135a89c7f6a734da02e778c34b`.
Those pins live in the committed lock files under
[`examples/compare/gitsitter/`](../examples/compare/gitsitter/).

## Two different kinds of Nix-native

The distinction from D67 matters before any timings do:

| Route | Native boundary | What follows from it |
| --- | --- | --- |
| Upstream flake | Derivation-native | A flake evaluates to a derivation. Other flakes can depend on it, Nix knows its runtime references, and normal substituters can serve its closure. |
| crane flake | Derivation-native | The same interoperation, with separate dependency artifacts used to shorten source-only rebuilds. |
| Cixfile via `cix build` | Store-native | Cix orchestrates networked `FETCH` and warm workspaces, then commits a store tree. Distribution goes through the Cix index. |
| Locked Cixfile via `buildCixfile` | Derivation-native, bounded | Pure Nix evaluation replays the checked-in cold plan as derivations. Other flakes can depend on the resulting `ITEM`, within the milestone boundary below. |

“Store-native” is deliberately narrower than “derivation-native”. Cix uses
Nix store paths and NAR identities, but `cix build` is an orchestration command,
not a flake evaluation interface. The Cix result in this example is a D68
`ITEM`: a pure, manifest-less tree containing `/bin/gitsitter`. It is not a
runnable Cix service or app.

There is also a current correctness gap in this exact example: Cix adds the
final tree without registering the dynamic library store references embedded
in its executable. Consequently, the reported Cix closure is incomplete and
is not evidence of a smaller distributable closure. The closure receipt below
shows the problem directly.

## Consuming a locked Cixfile from Nix

The CIP-94 library turns the reproducible subset of a checked-in `Cixfile` and
`Cixfile.lock` into an ordinary derivation, without installing or running cix
during evaluation or the build:

```nix
{
  inputs.cix-lib.url = "github:mathijshenquet/composix?dir=nix/lib";

  outputs = { self, cix-lib, ... }: {
    packages.x86_64-linux.default = cix-lib.buildCixfile {
      src = self;
      item = "my-package";
    };
  };
}
```

The root composix flake exports the same function as
`composix.lib.buildCixfile`. The lock carries a content-bound evaluation plan;
if the Cixfile changed, refresh and commit its lock before evaluating the
flake. A builder `FETCH` is one fixed-output derivation using the recorded
post-step NAR hash, and subsequent `RUN` steps are normal offline derivations.
Neither path nests bubblewrap: Nix supplies the build and network isolation,
while a namespace-free `proot` view recreates the cix filesystem skeleton and
synthetic uid 0. This keeps `buildCixfile` usable on hosts that disable
unprivileged user namespaces; the acceptance check realizes both derivation
classes under that policy and still requires byte-identical output.

Milestone 1 covers builder-less `ITEM` assembly with `IMPORT`, `COPY`, and
`FILE`, plus one `BUILDER` using `IMPORT`, `ENV`, `COPY`, `FETCH`, and `RUN`.
It rejects top-level fetches, artifact-valued `FROM`, multi-builder graphs,
and `SERVICE`/`APP` output at evaluation time. Builders importing the CIP-95
FHS loader surface are also rejected explicitly: mount-namespace loader
aliases cannot be reproduced by a plain Nix derivation yet. Use
`cix build --cold` for those definitions.

## What was authored

The upstream definition is not a hand-written low-level derivation. At the
pinned commit it uses `rustPlatform.buildRustPackage`, supplies `Cargo.lock`,
and declares `pkg-config`, `git`, OpenSSL, libgit2, and SQLite:

```console
$ upstream=$(nix flake archive --json github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd | jq -r .path)
$ sed -n '/mkPackage =/,/^      };/p' "$upstream/flake.nix"
      mkPackage = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "gitsitter";
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
        src = ./.;
        # The build sandbox has no .git, so hand build.rs the revision for `--version`.
        GIT_COMMIT_HASH = self.shortRev or self.dirtyShortRev or "unknown";
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
        nativeBuildInputs = with pkgs; [
          pkg-config
          git
        ];
        buildInputs = with pkgs; [
          openssl
          libgit2
          sqlite
        ];
      };
```

The crane fixture is conventional rather than code-golfed: common arguments,
`buildDepsOnly`, then `buildPackage`. Its committed definition is
[`crane/flake.nix`](../examples/compare/gitsitter/crane/flake.nix).

```console
$ sed -n '13,32p' examples/compare/gitsitter/crane/flake.nix
  outputs = { nixpkgs, crane, gitsitter, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      craneLib = crane.mkLib pkgs;
      commonArgs = {
        src = gitsitter;
        strictDeps = true;
        GIT_COMMIT_HASH = "29c8a2d";
        nativeBuildInputs = [ pkgs.pkg-config pkgs.git ];
        buildInputs = [ pkgs.openssl pkgs.libgit2 pkgs.sqlite ];
      };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    in
    {
      packages.${system}.default = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });
    };
}
```

The Cixfile uses a remote source binder, acquires Cargo dependencies in the
only networked step, compiles offline, and assembles a manifest-less `ITEM`.
Its committed definition is [`cix/Cixfile`](../examples/compare/gitsitter/cix/Cixfile).

```console
$ cat examples/compare/gitsitter/cix/Cixfile
FROM github:NixOS/nixpkgs/9cf7092bdd603554bd8b63c216e8943cf9b12512 AS pkgs
FROM github:mathijshenquet/gitsitter AS src

BUILDER build
IMPORT ${pkgs.bash} ${pkgs.cargo} ${pkgs.rustc} ${pkgs.pkg-config} \
    ${pkgs.gcc} ${pkgs.coreutils} ${pkgs.cacert} \
    ${pkgs.openssl} ${pkgs.openssl.dev} ${pkgs.libgit2} ${pkgs.libgit2.dev} \
    ${pkgs.sqlite} ${pkgs.sqlite.dev}
ENV GIT_COMMIT_HASH=${src.rev}
COPY ${src}/ .
FETCH mkdir -p .cargo && CARGO_TARGET_DIR=/tmp/cix-vendor-target cargo vendor --locked vendor > .cargo/config.toml
RUN cargo build --release --locked --offline

ITEM gitsitter
COPY ${build}/target/release/gitsitter /bin/gitsitter
```

Counting nonblank, non-comment lines avoids charging the upstream definition
for its explanatory comment:

```console
$ for f in "$upstream/flake.nix" examples/compare/gitsitter/crane/flake.nix examples/compare/gitsitter/cix/Cixfile; do awk 'NF && $1 !~ /^#/' "$f" | wc -l; done
38
30
13
```

LOC is only an authoring-weight proxy. The upstream author must understand a
flake, `buildRustPackage`, Cargo lock vendoring, and native versus build inputs.
The crane author adds source filtering and the dependency-artifact boundary,
but gets an explicit reusable dependency derivation. The Cixfile author works
with Docker-shaped binders and traced `COPY`/`FETCH`/`RUN` steps, while still
needing to understand package outputs such as `.dev`, `PKG_CONFIG_PATH`, the
network/offline split, and the difference between an `ITEM` and a runnable
artifact. Its shorter file does not erase those concepts.

## Measurement environment and protocol

The original rows were measured on 2026-07-31 UTC on Linux 6.17.0-40-generic,
x86_64, an AMD Ryzen 9 9950X3D (16 cores, 32 threads), with Determinate Nix
3.21.0 / Nix 2.34.6 and Cix 0.1.0. The Cix no-op and one-line-edit receipts
were re-run on 2026-08-02 with the copy-everything Cixfile and CIP-87 read-set
engine. The pre-existing Nix toolchain and native-library paths were retained
for every route. These are wall-clock observations, not a statistical
benchmark.

First, all three pinned definitions built successfully:

```console
$ nix build github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd --no-link -L
$ nix build path:./examples/compare/gitsitter/crane --no-link -L
$ target/debug/cix build examples/compare/gitsitter/cix#gitsitter
{
  "gitsitter": "/nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter"
}
```

The measured matrix was:

| Route | No-op | Subject-cold | One-line source change |
| --- | ---: | ---: | ---: |
| Upstream flake | 0.07 s | 28.82 s | 30.64 s |
| crane | 0.64 s | 37.81 s | 16.46 s |
| Cixfile | 0.07 s (2026-08-02) | 26.94 s | 14.46 s first-warm; 8.31 s / 8.84 s steady-warm (2026-08-02) |

### No-op

The baseline output already existed, and `/usr/bin/time` wrapped the ordinary
build command. The three commands were the build commands above with output
redirected to `/dev/null`.

```console
upstream_noop_seconds=0.07
crane_noop_seconds=0.64
cix_readset_noop_seconds=0.07
```

This Cix receipt was re-measured on 2026-08-02 (post trace-overhead round) on
the same host with the full memo hit already present. Its `--stats` result
reported zero Nix subprocesses and zero executed steps. The upstream and
crane measurements retain their original dates.

### Subject-cold

“Cold” did not mean an empty Nix store. It meant rebuilding the subject from
an empty compile result while keeping the common compiler and native inputs.
Substitution was disabled for each rebuilt subject.

For upstream, `--check` rebuilt the single `buildRustPackage` derivation,
including all Cargo crates:

```console
$ drv=$(nix path-info --derivation github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd)
$ /usr/bin/time -f 'upstream_cold_seconds=%e' nix-store --realise "$drv" --check --option substitute false
upstream_cold_seconds=28.82
```

For crane, the existing final output and its `gitsitter-deps` output were
deleted, then the final derivation was realised without substitutes. The
timing excludes deletion and store-root scanning:

```console
$ drv=$(nix path-info --derivation path:./examples/compare/gitsitter/crane)
$ deps_name=$(nix derivation show "$drv" | jq -r '.derivations | to_entries[0].value.inputs.drvs | keys[]' | rg gitsitter-deps)
$ deps_drv=/nix/store/$deps_name
$ nix store delete "$(nix-store -q --outputs "$deps_drv")" "$(nix-store -q --outputs "$drv")"
$ /usr/bin/time -f 'crane_cold_seconds=%e' nix-store --realise "$drv" --option substitute false
crane_cold_seconds=37.81
```

For Cix, the public cold boundary is explicit. It bypasses the saved build
workspaces and step memo:

```console
$ /usr/bin/time -f 'cix_cold_seconds=%e' target/debug/cix build --cold examples/compare/gitsitter/cix#gitsitter >/dev/null
cix_cold_seconds=26.94
```

On this subject and host, the three cold times are close enough that they
should not be generalized. Cix was fastest in this run; upstream had less
setup than crane; crane paid to create its separate reusable dependency
archive.

### One-line source change

[`warm.patch`](../examples/compare/gitsitter/warm.patch) changes only the CLI
description in `src/main.rs`. [`measure-warm.sh`](../examples/compare/gitsitter/measure-warm.sh)
resolves the pinned source, primes the unpatched routes, applies that exact
patch, and times each rebuild. Its current single-prime Cix receipt is the
first warm rebuild, not the steady state:

```console
$ examples/compare/gitsitter/measure-warm.sh
upstream_warm_change_seconds=30.64
crane_warm_change_seconds=16.46
cix_cold_control=green
cix_readset_warm_change_seconds=14.46
```

The 14.46 s first-warm result is not comparable to the prior two-prime 8.31 s
receipt. After that first edit, two further one-line edits in the same profiled
workspace measured 8.31 s and 8.84 s respectively. Both are steady-warm
receipts: FETCH is a memo hit and FETCH self-validation takes 0.217 s /
0.216 s. The third receipt adds an explicit `fetch-revert` timing marker,
which is absent, proving that the CIP-87 revert branch did not run on that
hit. The Cix invocation
also emitted this work receipt (abridged only to the two command steps):

```console
BUILDER build step 3 FETCH memo hit 5bbd3f90629a
BUILDER build step 4 RUN executed
{
  "stats": {
    "nixSubprocesses": 9,
    "steps": [
      {
        "kind": "FETCH",
        "name": "build:3",
        "status": "memo-hit"
      },
      {
        "kind": "RUN",
        "name": "build:4",
        "status": "executed"
      }
    ]
  }
}
```

Of the 8.31 s / 8.84 s steady-warm receipts, about 5.9 s is cargo's own floor
on this fixture: gitsitter's
`build.rs` declares `cargo:rerun-if-changed=.git/HEAD`, the staged source has
no `.git`, and cargo treats a missing rerun-if-changed file as permanently
stale, so build.rs + lib + bin recompile on every build — a no-Cix control
build in an identical sandbox takes 5.97 s for the same edit (and 5.9 s for a
no-op). The measured run compiled exactly one cargo unit, rehashed zero
read-set bytes (fingerprint fast-path), and spent 0.28 s total across its
nine Nix subprocesses.

The production Cixfile intentionally demonstrates a GitHub source binder.
For a controlled editable-source measurement, the script copies that Cixfile
to a temporary directory and changes only its source binder to `FROM . AS
src`. It removes the copied final-build memo, while retaining input and FETCH
pins, so one untimed local prime establishes the output workspace before the
first patch. A following patch is the steady-warm measurement. The upstream route
uses `overrideAttrs` for the patched `src`, and crane uses `--override-input`.
Thus all three see the same committed patch, while no benchmark-only local
binder is disguised as the shipped Cixfile.

The result shows the intended boundaries. Upstream's one derivation rebuilds
all crates. Crane reuses `cargoArtifacts`. The copy-everything Cixfile reuses
the networked vendoring step because its trace contains manifest and lock
content but only existence probes for Rust sources; RUN then uses its previous
end-state, and — since the 2026-08-02 trace-overhead round — staging and
replay preserve the inodes and mtimes of unchanged files, so cargo's own
fingerprints stay warm and only the edited unit recompiles. Capture remains
complete (file contents, directory listings, negative lookups, and writes are
all recorded via seccomp-BPF-filtered ptrace). Neither result is a claim
about arbitrary projects or clean builds.

## Output size and the missing-reference receipt

`nix path-info -sSh` reports NAR size first and recursive closure size second:

```console
$ nix path-info -sSh /nix/store/vzg59q1mrz38bddcn82jrrhx994c3ns2-gitsitter-0.2.1
/nix/store/vzg59q1mrz38bddcn82jrrhx994c3ns2-gitsitter-0.2.1  9.0 MiB  63.5 MiB
$ nix path-info -sSh /nix/store/l5phy4yvxi2wwqbafrx2kcy1yhwb4hlf-gitsitter-0.2.1
/nix/store/l5phy4yvxi2wwqbafrx2kcy1yhwb4hlf-gitsitter-0.2.1  9.1 MiB  63.6 MiB
$ nix path-info -sSh /nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter
/nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter  9.1 MiB  9.1 MiB
```

The first two results each register four direct references. The Cix result
registers none:

```console
$ nix-store -q --references /nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter | wc -l
0
$ ldd /nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter/bin/gitsitter | rg '/nix/store/'
        libgit2.so.1.9 => /nix/store/…-libgit2-1.9.1-lib/lib/libgit2.so.1.9
        libssl.so.3 => /nix/store/…-openssl-3.6.0/lib/libssl.so.3
        libcrypto.so.3 => /nix/store/…-openssl-3.6.0/lib/libcrypto.so.3
        libc.so.6 => /nix/store/…-glibc-2.42-47/lib/libc.so.6
```

The ellipses above shorten store hashes only; `ldd` printed real store paths.
Because those references are absent from Nix's metadata, 9.1 MiB is not the
closure of an independently usable dynamic executable. Until Cix preserves
or reconstructs them, this exact `ITEM` is not ready to move to a clean store.
Static or otherwise self-contained item trees do not have this particular
failure mode, but this comparison did not measure one.

## Determinism

The final outputs reproduced within each route, but the routes did not produce
the same bytes as one another:

| Route | Rebuild check | Stable result identity |
| --- | --- | --- |
| Upstream | `nix-store --realise "$drv" --check --option substitute false` passed | same output path and NAR hash `sha256:0asfab4r51f9z7a0b812j5ap43xp0di5p7ir6wwhxlgqjdyw1ygw` |
| crane | the same final-derivation `--check` passed | same output path and NAR hash `sha256:1mh3i90mikb8ckl7pkdfd837197rkhwhpckhfxv128bg3491bdmd` |
| Cixfile | normal, repeat, and `--cold` builds converged | `/nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter`, NAR hash `sha256:1mxy5zc081vywv0d2zj9n6i6kashlrrk83fk0z40vz0khfkmf73a` |

Cix initially used `cargo fetch`; its fetched Cargo home was not byte-stable.
The committed fixture now writes Cargo's vendor configuration directly during
the networked FETCH, while the IMPORT-derived environment supplies pkg-config
search paths. Normal, repeat, and `--cold` builds converge without a `.cargo`
cleanup-and-restore dance.

There is a second uncomfortable result. Crane's shipped final derivation
passed `--check`, but its roughly 154 MiB internal `gitsitter-deps` Cargo
artifact archive did not reproduce byte-for-byte under an independent
`--check`. That is not a failure of final gitsitter reproducibility, and it is
not honest to call every intermediate byte reproducible either. Hermetic
derivation inputs and byte reproducibility are related but distinct claims.

## “I want to distribute gitsitter — what now?”

The Cix-native route is to tag the one member, serve the index with its store
endpoint, then pull the tag into another index. A namespace is unnecessary
for a single-member build:

```console
publisher=$PWD/.tmp-publisher
consumer=$PWD/.tmp-consumer
CIX_STATE_DIR="$publisher" target/debug/cix build -t measured examples/compare/gitsitter/cix
CIX_STATE_DIR="$publisher" target/debug/cix serve --with-store --listen 127.0.0.1:18420
```

In another shell:

```console
$ CIX_STATE_DIR="$consumer" target/debug/cix pull 127.0.0.1:18420/gitsitter:measured --as gitsitter:measured
updated 1 tag(s)
$ CIX_STATE_DIR="$consumer" target/debug/cix ls -l
REF                 SYSTEMS       PATH                                                            UPSTREAM         AGE
gitsitter:measured  x86_64-linux  /nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter  127.0.0.1:18420  461s
$ test ! -e /nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter/cix-manifest.json
```

That receipt used separate publisher and consumer Cix state directories but
the same host Nix store; the displayed age is simply the value at capture. A
trial with a second local Nix store did **not**
pass: `cix pull` failed while parsing `nix path-info` JSON from that store.
Even if that parser issue were fixed, the missing runtime references above
mean this dynamic binary's clean-store execution has not been proven. There
is deliberately no `cix run` step: a D68 `ITEM` has no manifest and is not a
service or app. Tag lookup and byte serving work; independent installation of
this particular tool closure does not yet.

## Which route fits?

Use the upstream flake when the project already maintains a sound package and
you want the smallest amount of downstream machinery. It had the fastest
no-op here, participates directly in flakes and binary caches, and its simple
single derivation reproduced. Its trade-off is that a source edit rebuilds
the Cargo dependency compilation inside that derivation.

Use crane when you need a customized Rust package that remains an ordinary
derivation and want dependency/source build separation. It preserved flake
composition and a complete runtime closure while halving this warm rebuild
relative to upstream. The price here was more Nix authoring, the slowest cold
run, and a non-reproducible internal artifact archive despite a reproducible
final package.

Use a Cixfile when Docker-shaped build steps, an explicit networked
`FETCH`, and a stateful edit loop are more valuable than exposing a
derivation. It is 13 logical lines and keeps the networked FETCH warm across
this measured source edit, while normal versus `--cold` output identity held
after vendoring was normalized. The current completeness-first tracing path
made the measured edit slower than both derivation routes on this host.
The price is the D67 boundary: distribution uses the Cix index rather than
flake dependency edges. For this dynamically linked gitsitter build there is
also a concrete blocker, not merely a philosophical trade-off: final store
references are missing. Fix that before choosing this route to distribute the
tool to independent stores.

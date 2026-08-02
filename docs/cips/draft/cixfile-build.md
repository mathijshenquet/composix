# The Cixfile BUILD half: two designs against buildshape

> **Superseded by D39–D41, D47, and D50.** `BUILD rust`, `STAGE`, `OUTPUT`, and the
> engine-specific multi-output/service model below were not implemented. Cixfile now uses
> named `BUILDER` blocks and explicit `SERVICE`/`APP` artifacts connected by binders,
> documented in [The Cixfile](../../cixfile.md). This page remains as decision-record context.

*Status: superseded draft (pre-CIP exploration, filed here per the CIP README's
rule that refused/superseded drafts are records). Syntax on this page was
proposed Cixfile v2 syntax; none of it shipped.*

The useful test is not whether a BUILD syntax can compile one hello world. It is whether one
file can preserve the build shape in [`examples/build/proj1`](../../../examples/build/proj1/):

- one Cargo workspace with a shared internal library and three binary outputs;
- one dependency build reused by all three binary builds;
- a different source closure for each binary, without hiding Cargo's workspace manifests;
- one separately locked pnpm build whose `dist/` becomes a static item; and
- four selectable composix items, three with a service and one containing only static files.

The handwritten flake proves that this shape is buildable. The question here is how much of
its knowledge belongs in Cixfile itself.

## Common multi-output semantics

Both variants use the same output model:

- `OUTPUT <name>` starts an item assembly block. `COPY` and `SERVICE` directives that follow
  belong to that output until the next `OUTPUT`.
- Output names match `[a-z][a-z0-9-]*` and must be unique. A stage name and output name may
  match, but they are separate namespaces.
- `DEFAULT <name>` makes `cix build .` select one output. `cix build .#api` selects exactly
  `api`; `cix build . --all` realizes every declared output. An unknown selector fails before
  evaluation and prints the available names.
- `cix build .#api -t local/api` tags only the selected item. With `--all`, `-t local/example`
  produces `local/example-api`, `local/example-worker`, and so on; it never makes one tag
  ambiguously point at several items.
- A `SERVICE` belongs only to its enclosing output. Service names need only be unique within
  that output. An output may have no services, which is how `frontend` remains a static item.
- `COPY --from=<stage> <src> <dst>` copies or links a declared stage result into the current
  item. Stage paths are logical paths, not host paths or arbitrary Nix store paths.

After assembly, each output is an ordinary composix item with its own `cix-manifest.json`.
Selecting an output is a build-time concern; selecting `item#service` remains the existing
run-time concern.

## Variant A — inline minimal magic

Variant A adds two recognized builders and Docker-like stages. The entire new build surface
is:

```text
STAGE <name>
BUILD rust <source> [--deps-only | --bin=<name> --deps-from=<stage>]
                   [--keep=<workspace-member,...>]
BUILD pnpm <source> --script=<name> --export=<directory>
COPY --from=<stage> <src> <dst>
OUTPUT <name>
DEFAULT <output>
```

`BUILD rust` has fixed Cargo/crane behavior:

- `Cargo.lock` and `rust-toolchain.toml` are required at the source root and are inputs
  automatically. Absence is an error, not an invitation to choose an ambient toolchain.
- `--deps-only` is crane's dependency-only build over the full Cargo source set.
- `--bin` builds from the workspace root, with the named dependency stage. It does not add a
  package selector, because changing Cargo's resolution scope can defeat artifact reuse.
- `--keep` names the workspace members whose full source is visible to that binary build.
  Every member manifest and required target entrypoint remains visible so Cargo can resolve
  the workspace. The compiler rejects names not present in `workspace.members`.
- The builder admits Cargo manifests, lock/toolchain files, and ordinary Rust source. There
  is deliberately no general include/exclude expression language.
- Locked Cargo vendoring is automatic. There is no hook for editing a vendored dependency.

`BUILD pnpm` likewise requires `package.json` and `pnpm-lock.yaml`, takes the pnpm version
from `packageManager`, runs only the named package script, and exports only the named
directory. Its fixed-output dependency hash is generated into `Cixfile.lock`; the actual
build is offline.

### Full Variant A Cixfile

```dockerfile
CIXFILE 2

STAGE rust-deps
BUILD rust rust/ --deps-only

STAGE api-build
BUILD rust rust/ --bin=api --deps-from=rust-deps --keep=common,api

STAGE worker-build
BUILD rust rust/ --bin=worker --deps-from=rust-deps --keep=common,worker

STAGE dashboard-build
BUILD rust rust/ --bin=dashboard --deps-from=rust-deps --keep=common,dashboard

STAGE frontend-build
BUILD pnpm frontend/ --script=build --export=dist

OUTPUT api
COPY --from=api-build bin/api bin/api
SERVICE api
START bin/api

OUTPUT worker
COPY --from=worker-build bin/worker bin/worker
SERVICE worker
START bin/worker

OUTPUT dashboard
COPY --from=dashboard-build bin/dashboard bin/dashboard
SERVICE dashboard
START bin/dashboard

OUTPUT frontend
COPY --from=frontend-build dist/ www/

DEFAULT api
```

The stages form a DAG, not mutable filesystem layers. `rust-deps` is built once and is an
input to three independent binary derivations. Reordering the binary stages has no effect.
`COPY` is the only operation that moves a stage result into an item, so build-only inputs do
not enter a service closure accidentally.

### What Variant A can and cannot express

For the buildshape stub, it covers the important techniques exactly:

| Handwritten-flake technique | Variant A |
| --- | --- |
| rust-overlay toolchain selected from `rust-toolchain.toml` | Built into `BUILD rust` |
| one crane `buildDepsOnly` result shared by binaries | `--deps-only` plus `--deps-from` |
| per-binary source closures | Manual, checked `--keep` lists |
| retain all workspace manifests and minimal non-selected targets | Builder invariant |
| build a binary from the workspace root | Builder invariant |
| filtered frontend source | Builder excludes `node_modules` and the export directory |
| fixed-output pnpm dependency store | Built into `BUILD pnpm` and recorded in the lock |
| several package outputs | `OUTPUT`, `DEFAULT`, and output selectors |

The fixed surface also draws a hard line:

- standard lockfile-based vendoring fits; project-specific rewrites of a vendored checkout do
  not;
- Rust source plus manifests fits; unusual generated inputs or extra source file families do
  not;
- ordinary native builder options could become a small, reviewed set of flags, but arbitrary
  build inputs, environment variables, wrapper construction, and post-install scripts do not;
- one named pnpm script and one exported directory fit; a bespoke sequence of frontend build
  phases does not.

Those are `.nix` cases. Adding `RUN`, shell hooks, raw filter predicates, or Nix fragments to
make them fit would turn the simple syntax into a second, worse Nix language.

### Locking and errors in Variant A

`Cixfile.lock` pins nixpkgs, crane, rust-overlay, the Cix BUILD compiler version, and every
fixed-output dependency result. Cargo and pnpm lockfiles remain source inputs. The
`rust-toolchain.toml` channel and `packageManager` version are mandatory, while the overlay
and package implementations that realize those versions come from pinned inputs. A lock
update is explicit:

```console
cix build .#api --update-lock
```

Changing a BUILD flag changes the stage derivation. Changing only `worker`'s kept source
changes `worker-build`, not `api-build`; changing the shared lock or shared library changes
the common dependency stage and all consumers.

Errors should expose both the Cixfile line and the underlying builder phase:

```text
api-build: BUILD rust --bin=api (Cixfile:7)
  cargo failed while compiling workspace member "api"
  possible source-filter omission: member "common" is used but absent from --keep
  rerun with: cix build .#api --show-build-log
```

The hint can be wrong, but the stage, builder, selector, and full Cargo log must remain
available. A fixed builder makes these hints feasible because the compiler understands the
operation that failed.

## Variant B — versioned build plugins

Variant B keeps Cixfile small by moving ecosystem knowledge behind `USE`. BUILD expressions
are typed Unix-like pipelines: the left side produces one value, `|` passes that value to a
plugin, and named stages can be supplied as additional inputs.

### Full Variant B Cixfile

```dockerfile
CIXFILE 2

USE cargo cixpkgs:cargo@1.0.0
USE pnpm cixpkgs:pnpm@1.0.0

BUILD rust-source = SOURCE rust/ LOCK Cargo.lock rust-toolchain.toml
BUILD rust-deps = rust-source | cargo mode=deps
BUILD api-build = rust-source | cargo mode=bin bin=api deps=rust-deps keep=common,api
BUILD worker-build = rust-source | cargo mode=bin bin=worker deps=rust-deps keep=common,worker
BUILD dashboard-build = rust-source | cargo mode=bin bin=dashboard deps=rust-deps keep=common,dashboard

BUILD frontend-source = SOURCE frontend/ LOCK package.json pnpm-lock.yaml
BUILD frontend-build = frontend-source | pnpm script=build export=dist

OUTPUT api
COPY --from=api-build bin/api bin/api
SERVICE api
START bin/api

OUTPUT worker
COPY --from=worker-build bin/worker bin/worker
SERVICE worker
START bin/worker

OUTPUT dashboard
COPY --from=dashboard-build bin/dashboard bin/dashboard
SERVICE dashboard
START bin/dashboard

OUTPUT frontend
COPY --from=frontend-build dist/ www/

DEFAULT api
```

`SOURCE` is a compiler builtin, not a plugin. It creates an immutable source value and names
the lock/config files whose bytes must participate in the plan. Paths after `LOCK` are
relative to the source root. The same source value may feed several pipelines.

### Plugin contract

A plugin is a composix store item with:

```text
bin/cix-build-plugin
share/cix-plugin/manifest.json
```

The manifest contains the plugin name and semantic version, protocol versions, accepted
input and output kinds, a JSON Schema for parameters, required file roles, supported target
systems, and the digest of its planner executable. `cix` rejects a plugin whose manifest and
lock entry disagree.

Protocol v1 has two operations:

1. `describe` prints the manifest as canonical JSON.
2. `plan` reads one canonical JSON request from standard input and writes one canonical JSON
   response to standard output. Diagnostics go to standard error as newline-delimited JSON.

The request is conceptually:

```json
{
  "protocol": 1,
  "source": {
    "storePath": "/nix/store/…-source",
    "narHash": "sha256-…",
    "lockedFiles": ["Cargo.lock", "rust-toolchain.toml"]
  },
  "params": {
    "mode": "bin",
    "bin": "api",
    "keep": ["common", "api"]
  },
  "inputs": {
    "deps": {
      "stage": "rust-deps",
      "kind": "cargo-artifacts",
      "drvPath": "/nix/store/…-rust-deps.drv"
    }
  },
  "target": "x86_64-linux"
}
```

The plugin receives no ambient working tree, environment variables, credentials, clock, or
network. It can read the immutable source and declared named inputs. Parameter ordering is
canonicalized by `cix`; undeclared parameters are errors.

The response is not raw Nix. It is a typed derivation plan containing:

- a named builder adapter from the pinned Cix compiler (`crane-v1`, `pnpm-v1`, or another
  protocol-defined adapter);
- content-addressed source-filter rules;
- declared derivation inputs and fixed-output fetches;
- adapter arguments, environment values allowed by that adapter, and logical output paths;
- the primary output kind and any named side-output kinds; and
- structured diagnostics with source locations and stable error codes.

`cix` validates this plan against the plugin manifest and the adapter schema, then generates
the Nix derivation. Neither a Cixfile author nor a plugin may inject a Nix expression or a
shell phase through protocol v1. A plugin that genuinely needs a new primitive requires a
reviewed adapter addition—or the project graduates to `.nix`.

This restriction matters. If plugins returned arbitrary Nix text, `USE` would be a
less-auditable spelling of `import`, error locations would be lost, and D20a's “no raw Nix
passthrough inside Cixfile” would be technically true but substantively false.

### Composition

Pipeline values are typed build descriptions, not byte streams:

```text
source-tree | cargo -> cargo-artifacts | cargo -> directory-tree
source-tree | pnpm -> directory-tree
```

Each plugin has one primary input and one primary output so `|` remains legible. Extra DAG
edges are explicit named parameters such as `deps=rust-deps`; they are never discovered from
the filesystem or environment. A later generic plugin could consume a `directory-tree`:

```text
BUILD checked-frontend = frontend-source | pnpm script=build export=dist | static-check
```

The compiler type-checks the full graph before asking Nix to evaluate anything. Cycles,
missing stages, wrong value kinds, undeclared paths, and unknown parameters are Cixfile
errors.

### Versioning and distribution

`cixpkgs:cargo@1.0.0` is a human-facing registry coordinate, not the reproducibility anchor.
On first resolution, `Cixfile.lock` records:

- the cixpkgs index revision and content hash;
- the plugin's immutable store item reference and NAR hash;
- its semantic version, protocol version, and manifest digest; and
- the builder-adapter and compiler versions used to validate its plan.

Plugins are themselves composix items. They can be built, inspected, signed, mirrored,
tagged, and garbage-collected using the same machinery as other items. A registry tag may
move; the lock entry cannot. `--update-lock` is the only operation that follows a newer tag.
An out-of-tree plugin can be addressed by an item reference instead of cixpkgs, but is locked
the same way.

Semantic versioning communicates intent: a changed plan for identical inputs requires at
least a minor release; a schema or output-kind incompatibility requires a major release.
Reproducibility still comes from the hash, not from trusting version etiquette.

### Failures in Variant B

The extra boundary creates extra failure classes. They need distinct reporting:

```text
api-build (Cixfile:7)
  cargo@1.0.0 plan failed [CARGO_SOURCE_CLOSURE]
  parameter: keep=["common","api"]
  diagnostic: workspace dependency is outside the selected source closure
```

Protocol corruption, a manifest mismatch, an unsupported system, plan validation, Nix
evaluation, and builder execution must be labeled separately. Plugin stderr that is not
valid diagnostic JSON is preserved under “plugin stderr,” never discarded. After planning,
the generated derivation and its source map are inspectable with:

```console
cix build .#api --show-plan
```

The likely bad failure mode is indirection: users know Cargo, but now must understand a
plugin's vocabulary, a protocol error, and the eventual crane/Cargo error. Good structured
diagnostics reduce that tax; they do not eliminate it.

The standard cargo plugin can reproduce the stub's source closures, dependency reuse, and
workspace-root builds. It can also add generally useful, versioned policies later. It should
not accept arbitrary vendor-tree scripts. A project-specific vendor rewrite either becomes
an audited, schema-level policy with broader evidence or stays in `.nix`.

## Adversarial comparison

| Criterion | Variant A: fixed builders | Variant B: plugins |
| --- | --- | --- |
| Expressiveness for buildshape | Complete | Complete |
| Expressiveness beyond known shapes | Intentionally low | Higher only where the typed protocol and adapters already reach |
| Per-binary source filtering | One explicit `--keep` knob plus compiler invariants | Cargo plugin parameters and plan |
| Vendor special cases | Standard vendoring only | Standard plugin policy; arbitrary rewrites still excluded |
| Magic budget | Small surface, substantial but centralized builder behavior | Small Cixfile surface, larger distributed behavior and protocol |
| Determinism | Compiler/builders and fixed outputs pinned in one lock | Compiler, adapter, registry, plugin item, manifest, and fixed outputs all pinned |
| Supply-chain surface | nixpkgs, crane, overlay, compiler | All of Variant A plus each plugin publisher and registry resolution |
| Failure legibility | Compiler knows every operation and can annotate it directly | Potentially excellent structured diagnostics, but more failure layers |
| Independent ecosystem evolution | Requires a composix release | Plugin releases can move independently |
| Local extension | `.nix` | A protocol-conforming plugin, if existing adapters suffice; otherwise `.nix` |
| Graduation | Replace BUILD stages with a sibling `default.nix`; SERVICE can remain conceptually unchanged | Inspect the plan, then encode the graph directly in `.nix` |

### Expressiveness versus the handwritten flake

Both variants cover the stub. Neither covers the full class of techniques that a handwritten
flake can express: arbitrary source predicates, dependency-vendor surgery, custom native
toolchains, generated assets outside the recognized source model, wrapper construction,
platform-specific package sets, or arbitrary checks and deployment outputs.

That gap is healthy if it stays visible. The Cixfile is the paved road; `.nix` is not a
failure state. A misleading escape valve—`RUN`, `NIX <<EOF`, plugin-returned Nix text, or a
shell-valued parameter—would make the paved road impossible to reason about.

### Determinism is more than having a lockfile

Variant A has fewer moving parts to lock and a smaller evaluator trust base. Variant B can be
equally reproducible only if the plugin executable, manifest, adapter, compiler, registry
snapshot, source, declared files, parameters, and named inputs all enter the plan hash. If
even one is ambient, “plugins as Unix tools” becomes host-dependent execution.

Neither design should silently repair a stale source lockfile. Cargo uses `--locked`; pnpm
uses its frozen lock mode. Updating `Cargo.lock`, `pnpm-lock.yaml`, `Cixfile.lock`, or a
plugin coordinate is a separate, reviewable change.

### Graduation to `.nix`

`cix build --show-plan` should work in both variants. `cix build --emit-nix build.nix` may
write the generated expression for inspection and migration, but Cixfile cannot embed it.
The emitted file must be readable and standalone against the same lock inputs, with stage
names retained in derivation names.

Graduation happens when a project needs semantics, not merely syntax, outside the paved
road. The user keeps the same output names and service contracts, replaces generated build
derivations with explicit Nix, and can compare output closures during the transition.

## Recommendation

Implement Variant A first.

The buildshape exercise found a compact fixed surface that preserves the valuable behavior:
shared dependency artifacts, per-binary invalidation, locked toolchains, an offline frontend
build, and multiple independently runnable items. Variant A puts the unavoidable magic in
one versioned compiler where error messages, locks, and invariants can be tested together.
It matches the current design principle that Cixfile recognizes a small set of blessed
builders and otherwise yields to `.nix`.

Variant B is attractive as an eventual distribution boundary, but protocol v1 would mostly
relocate the same cargo and pnpm code while adding registry trust, protocol compatibility,
typed-plan evolution, and another diagnostic layer. “Plugins are items” is conceptually
clean; it is not yet evidence that users need a plugin ecosystem.

Evidence that would change the recommendation:

- three or more ecosystem builders need release cadences that cannot reasonably follow the
  Cix compiler;
- the same non-core build policy appears in several unrelated projects and fits a typed
  adapter without shell or raw Nix;
- a prototype proves that plugin plan hashes are complete and stable across machines;
- plugin-originated failures are at least as legible as fixed-builder failures in user tests;
  and
- independent plugin distribution materially reduces maintenance without expanding the
  trusted or ambient input surface.

Until then, the smaller language is the stronger promise.

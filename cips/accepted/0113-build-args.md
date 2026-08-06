# build-args — parameterizable Cixfiles: the design space (v4)

Status: **draft v4** (2026-08-06; v2 was the prior-art round after v1's
state-skew rejection; v3 folded in Mathijs's review of v2: partial
locking yes, coupled args probably not; v4 works out args×tagging as a
declared `TAG` line after Mathijs's v3 review called the CLI-interpolation
sketch janky. Lands together with nodes-and-edges + phase-blocks as one
language epoch.)

## 1. The problem

One Cixfile, many variants: a CI matrix building N versions, a
version-stamped release (the gitea corpus pattern), a feature toggle.
Docker serves this with `ARG` + `--build-arg`. The D32-era refusal had
one real argument: an ambient CLI channel breaks "Cixfile + lock is
the whole truth" — a build would depend on values recorded nowhere,
and replay (`--cold`, buildCixfile) could not reproduce it.

v1 proposed recording the override in the lock. That fails differently:
the lock becomes *mutable state tracking the last invocation* instead
of a pure function of the file. Which variant you built last changes
your diff; parallel builds race; audit reads history, not truth.

And the cost of doing nothing is concrete: users who need variants
will *generate* Cixfiles with string templating — the Helm failure
mode — which is strictly worse than any ARG design, because generated
text bypasses every guarantee the language carries.

## 2. Prior work

- **Docker ARG** — ambient and effectively unrecorded (values leak
  into `docker history`, nothing authoritative); infamous scoping
  rules; no lock concept to skew. The convenience datum, not a design.
- **Helm** — parameters over string-templated YAML. The canonical
  cautionary tale for stringy generation (whitespace/indent bugs, type
  coercion). But note *where it records*: computed values are stored
  per **release** — per instantiation, server-side — never in the
  chart source.
- **Kustomize** — the explicit anti-template position: no parameters;
  every variant is an overlay **file**, committed and diffable. Whole
  truth preserved; cost is file proliferation (our `--file` twins are
  this route).
- **Nix flakes** — refused ambient `--arg` for purity. Variants became
  **enumerated outputs**: the file declares the matrix, every cell is
  addressable by name, one lock covers all cells. CLI input never
  introduces values that aren't in source; `--override-input` exists
  but writes the lock as a visible move.
- **Cargo features** — build parameterization with a **union lock**:
  Cargo.lock records the dependency graph for *all* feature
  combinations; selection is per-invocation and unrecorded, yet builds
  are reproducible because resolution is deterministic given
  (lock, selection).
- **Terraform** — free-form variables, but resolved values are
  recorded per instantiation (plan/state) and skew is surfaced as a
  plan diff before apply.
- **Bazel** — configuration is part of every action's cache key;
  per-combination keying handled invisibly by content addressing.

Three lessons: (a) never string-template a structured file; (b) the
two honest recording models are *enumerate variants in source*
(kustomize, flake outputs) or *record values per instantiation* (Helm
release, Terraform state) — nobody records selection in shared
source-side state, which is exactly the skew v1 tripped on; (c) one
lock can honestly cover a whole parameter space when entries are keyed
by resolved request (cargo) — and our locks already have this
property: fetch entries key on a hash of the **resolved** statement
(caddy's lock carries coexisting entries for one source line with
different resolved hashes today).

## 3. Recommendation — and the routes considered

**Route A (recommended): closed-matrix ARG.** The Cixfile declares an
ARG together with its *finite* set of allowed values (syntax open,
e.g. `ARG VERSION = 1.24.2 | 1.25.1`). Consequences, each mechanical:

- The **file declares the whole matrix**; `cix build --arg NAME=value`
  merely *selects* a declared cell — the CLI can never introduce a
  value that is not in source. Undeclared value → parse-style error
  listing the declared ones.
- The **lock is partial, per cell** (Mathijs, v2 review: locking the
  whole matrix eagerly is intense; obvious semantics instead). A cell
  you build for the first time pins through the ordinary
  first-build/TOFU flow and its entries are appended, keyed by the
  resolved statements (keys already distinguish these — caddy's lock
  carries coexisting entries for one line today); a cell already
  pinned replays read-only; `--all-args` exists for CI to pin or build
  the full declared matrix in one move. Precision about the invariant:
  the lock's *coverage* (which cells are pinned) is chosen and visible
  in review; its *content per covered cell* is a pure function of the
  file. No mutation skew: cells append independently, they never
  rewrite each other.
- The **manifest records the selection** (Helm/Terraform lesson:
  instantiation-side recording), so a built artifact answers "which
  variant am I?" via `cix inspect`.
- Open-ended values stay refused, on principle: arbitrary external
  input is *generation*, not selection — see Route C.

**Route B (considered, not preferred): open ARG + composite lock.**
Cargo-style union lock, first-build-pins per combination via the TOFU
flow. Append-only growth softens v1's skew (append, not mutate), but
the lock stops being a function of the file — which combinations are
pinned becomes invisible state, CI needs pre-pinned combos anyway, and
retired combinations' entries have no principled GC. The extra freedom
buys exactly the use cases that belong to generation.

**Route C (complement, not alternative): a generation affordance.**
Whatever happens with ARG, open-ended parameterization (arbitrary
versions, computed config) is generation — and it must not be string
templating. The typed route exists in-system: emit Cixfile text
through the formatter/AST (the `.nix` escape hatch + buildCixfile
already put nix in reach as the generator language; per-variant twins
get per-variant lock *files*, which is the kustomize model with a
generator behind it). Near-term this is a documented idiom plus a
stability guarantee on canonical formatting; a first-class `cix gen`
belongs to a later tooling era (same shelf as `docker init`'s ⏳).

### Coupled args (considered; probably unnecessary)

Docker matrices often need value *pairs* — `VERSION` and its
`DOWNLOAD_SHA256` travel together (the adminer pattern). Do we need
coupled enumeration, so invalid cross-product cells cannot be
selected? Mostly no, for a structural reason: **the lock already is
the coupling mechanism.** Docker needs the hash as a second build-arg
only because it has no lock; in cix the per-cell pin (TOFU or EXPECT)
records exactly the url→hash binding for each declared version, so
the hash never becomes an arg at all — adminer's `*_SHA256` variables
dissolve into lock entries. The residual edge is the CIP-102 idiom of
verifying a *vendor-published* checksum inside RUN (authorial trust,
not integrity): per-version vendor checksums would want a pair table.
Rare enough to leave out of v1 of the feature; if it bites, the
answer is enumerating pairs as single values rather than a coupling
construct across args.

### Args × tagging

DEFERRED OUT at adoption (Mathijs, 2026-08-06): the worked TAG-line
design moved intact to `cips/deferred/build-args-tag.md` for a later
round. CIP-113 ships without any TAG surface; tagging stays CLI-side
(`--tag` / `cix tag`). Under `--all-args`, per-cell tags are given on
the CLI or applied afterwards via `cix tag` with `cix inspect`
resolving the cell.

## 4. Open questions

Resolved in review (Mathijs, 2026-08-05): the lock is partial per
cell with first-build-pins semantics — eager full-matrix locking is
rejected as too heavy; coupled args stay out of v1. (2026-08-06): the
epoch trio lands together; TAG deferred out entirely (see §3).

## 5. Decision (adopted 2026-08-06, Mathijs)

Route A adopted: closed-matrix ARG, partial per-cell lock with
first-build-pins, manifest-recorded selection, `--all-args` for CI,
Route C's generation idiom documented as the open-ended fallback.
Calls closed at adoption:

- **Syntax follows CIP-111's juxtaposition decision**:
  `ARG VERSION from 1.24.2 1.25.1` — a bare word-list matrix, no
  bracket literal, the same enumeration shape future LET-lists use.
- **Default: the first declared value** (gap filled under standing
  delegation: simplest rule, matches the docker mental model, keeps
  plain `cix build` meaningful for every matrix file). A no-default
  `required` marker is deferred until a real case demands it.
- **TAG deferred out** (Mathijs: "doen we tzt") — design parked
  intact in `cips/deferred/build-args-tag.md`; no TAG surface in
  this CIP.
- **Twins vs args guidance** as proposed: args for same-shape
  variants of one artifact; twins for genuinely different build
  shapes. Acceptance test stays the gitea version-stamp case plus a
  CI `--all-args` pin of a declared matrix.

Changelog:
- 2026-08-06 — adopted as part of the epoch (with CIP-111/112).

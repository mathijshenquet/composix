# pnpm-wall — the package-manager fetch problem, dissected

Status: **draft v2** (2026-08-05; expanded from the expand1 CIP-light
per Mathijs — four chapters, full analysis. This is where the
"traced sandbox replaces the *2nix genre" claim gets tested where it
matters most: node is where real Dockerfiles live.)

## 1. The problem

Several corpus failures look like one "pnpm wall" but are materially
different things, and treating them as one ecosystem failure would
produce bad language changes. The precise exhibits:

1. **Homer — a false positive, and the most instructive exhibit.**
   Staging failed with `UNABLE_TO_GET_ISSUER_CERT_LOCALLY` and looked
   like a registry hang. The independent recheck with `cacert`
   imported and a FETCH-traced pnpm data directory completed the
   offline Vite build. Lesson: a missing TLS-trust prerequisite can
   masquerade as a pnpm wall — and clients retry-with-backoff on
   certificate errors, so the symptom is a stall, not a clean error.
2. **Dozzle — the undiagnosed hang.** `pnpm fetch --ignore-scripts`
   exceeded its 300-second bound without producing anything
   (receipt.md). Given exhibit 1, the cacert class is the prime
   suspect; IPv6-AAAA-before-v4 connection timeouts (per connection,
   times hundreds of packages) are the second. Not yet diagnosed —
   see Recommendation item 1.
3. **Verdaccio — CONSUMED volatility, the hard core.** Cold replay
   observes a volatile pnpm root read set: the store's SQLite
   index/metadata files differ per fetch AND are genuinely read by
   the subsequent step. Read-set reduction (CIP-102's rescue) only
   removes UNCONSUMED volatility; here the nondeterministic bytes are
   load-bearing, so no honest pin exists for the tree as fetched.
4. **Directus — upstream incoherence, not our wall.** At the pinned
   revision, root `package.json` and `pnpm-lock.yaml` do not match,
   so `--frozen-lockfile --offline` refuses before deploy. pnpm's
   validation is doing its job on incoherent upstream data.
5. **Filestash (adjacent, no pnpm) — snapshot scale.** The first
   Go-module FETCH seals ~2.7 GiB / 69k files and exceeds 20 minutes:
   the cost is snapshot-taking time. Related but distinct:
   existence-only observations (`Absent`/`*Exists`) are legitimately
   incompressible under CIP-99 (a subtree narHash cannot witness an
   absence), and incomplete traces (it-tools' bounded run) cannot
   aggregate until a complete corrected build exists.

Why it matters: package managers are themselves build systems — they
interleave network, cache mutation, and resolution in ways that
produce no stable, boundable artifact tree by default. The node
ecosystem is the largest population of real Dockerfiles; cracking
this natively is what makes the traced-sandbox thesis ("no *2nix
translators needed") hold where the translator industry is biggest.

## 2. Prior work

- **The *2nix genre** (node2nix, npmlock2nix, crane, naersk,
  poetry2nix, …): translate the ecosystem's lockfile into nix
  fetches. Works, but is a per-ecosystem, per-lockfile-version
  maintenance treadmill — the thing cix's traced sandbox exists to
  obsolete. Keep as the fallback shape, never the default.
- **pnpm's own architecture** — the decisive prior work: the store is
  content-addressed underneath (`~/.pnpm-store/v3/files/` keyed by
  content hash); the SQLite index and metadata are DERIVED state.
  pnpm supports a real two-phase flow: `pnpm fetch` (lockfile-only,
  populates the store) then `pnpm install --offline`. The CAS part is
  pinnable; the volatile parts are regenerable.
- **BuildKit cache mounts** (`RUN --mount=type=cache`): Docker's
  answer — a declared, persistent, UNKEYED cache outside the layer
  model. Maps to the recorded `WITH CACHE` direction in
  nodes-and-edges: an edge kind that says "this path is cache —
  exclude it from read-set evidence, persist it across builds".
- **Yarn offline mirrors / npm `--prefer-offline` +
  `npm ci`**: the ecosystem's own reproducibility affordances all
  converge on "materialize the packages once, install offline from
  the materialization".
- **Nix FODs**: fixed-output derivations legitimize "network allowed,
  output pinned" — cix's FETCH already is this shape (CIP-94
  snapshotNarHash); the gap is only WHICH bytes get pinned when the
  tree contains derived volatile state.

## 3. Recommendation

Four legs, ordered by evidence-before-mechanism:

1. **Diagnose the hang class first (no language change).** Rerun the
   dozzle fetch under the bound with: `cacert` imported (the homer
   fix), pnpm verbose/network logging, and strace/ss capture. If the
   cacert class explains it, the fix is a migrate.md teaching line
   ("ecosystem fetches need `${pkgs.cacert}` imported — TLS-trust
   errors masquerade as hangs") plus a diagnostic: when a FETCH times
   out and the trace shows repeated TLS handshake failures, say so.
   Check IPv6 fallback behavior in the same session.
2. **Two-phase store-as-artifact for pnpm/npm.** The canonical
   translation for ecosystem fetches becomes: FETCH runs
   `pnpm fetch` and the PINNED ARTIFACT is the content-addressed
   store portion; derived indexes are either excluded from the
   snapshot and regenerated offline at install (preferred — they are
   derived state, like nix's own sqlite caches) or normalized before
   sealing. The install step runs `--offline` against the pinned
   store. This dissolves exhibit 3's consumed-volatility: what the
   install consumes is the CAS (stable) plus an index it rebuilds
   itself. Needs a spike: verify pnpm regenerates its index from a
   bare CAS store across versions, and how npm's cacache compares.
3. **`WITH CACHE` lands as designed in nodes-and-edges** (recorded
   direction there): declared cache paths outside read-set evidence,
   persisted across builds. This addresses the scale face (exhibit
   5): module/package caches stop being sealed at all — the seal
   covers the project tree, not the ecosystem's cache. Also the
   honest answer for Go's module cache (filestash) without any Go
   special-casing.
4. **Upstream-incoherence stays honest** (exhibit 4): a
   manifest/lock mismatch at the pinned revision is an upstream
   defect; the diagnostic should name it as such ("pinned revision's
   package.json and pnpm-lock.yaml disagree — upstream defect; pin a
   coherent revision or record the gap"), never auto-fall-back to a
   non-frozen install (which would mutate the lock and lie about
   reproducibility).

Explicitly rejected: a global pnpm exception; weakening network
isolation; translating lockfiles by default (*2nix fallback stays a
fallback); pinning volatile trees as-is (a pin that cannot replay is
worse than no pin).

## 4. Open questions

- Does pnpm rebuild its store index deterministically from a bare CAS
  across the versions the corpus pins? (Spike, exhibit-driven:
  verdaccio + dozzle.) Same question for npm's cacache.
- Where does the pinned store live in the artifact model — a FETCH
  output tree like today, or does `WITH CACHE` subsume it entirely
  (cache persisted, nothing pinned)? The difference is evidence:
  a pinned store replays cold; a cache does not. Likely answer: pin
  for corpus/receipt cases, cache for dev-loop speed — both exist.
- Seal-time engineering for large stores (exhibit 5): parallel
  hashing, or does CAS-portion pinning shrink the seal enough on its
  own?
- Does the homer cacert lesson generalize into a prelude convention
  (migrate.md teaches `IMPORT ${pkgs.cacert}` for any networked
  FETCH) or a language affordance (FETCH implies cacert in its
  sandbox)? The latter touches determinism claims — needs its own
  taste call.

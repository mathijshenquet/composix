# pnpm-wall — the package-manager fetch problem, dissected

Status: **draft v3** (2026-08-06; evidence update from the pnpm-wall spike;
expanded from the expand1 CIP-light
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
2. **Dozzle — another cacert masquerade.** An actual cix IMPORT-union
   A/B made the class conclusive. Without `cacert`, the clean FETCH
   timed out after 180 seconds with no CAS files; its syscall trace
   ended in repeated failed OpenSSL certificate-directory lookups.
   With `${pkgs.cacert}`, exact pnpm 11.17.0 completed both FETCH
   stability probes and exited 0. A separate network-instrumented run
   finished in 6.6 seconds with 52 IPv6 and 38 IPv4 connect calls,
   every package fetch on attempt 1, and no TLS errors. This was not
   IPv6 fallback; missing trust material made the client look hung.
3. **Verdaccio — CONSUMED volatility, the hard core.** Cold replay
   observes a volatile pnpm root read set: the store's SQLite
   index/metadata files differ per fetch AND are genuinely read by
   the subsequent step. Read-set reduction (CIP-102's rescue) only
   removes UNCONSUMED volatility; here the nondeterministic bytes are
   load-bearing, so no honest pin exists for the tree as fetched. The
   store spike sharpened this: pnpm 11.1.2 and 11.17.0 produce stable
   `files/` CAS trees, but neither can resolve packages offline from
   those bytes without its volatile SQLite package index.
4. **Directus — coherent upstream; incomplete offline metadata.** The
   pinned `package.json` and `pnpm-lock.yaml` exactly match git revision
   `b1d7a45a77661fd13928a53448c06649f36b56f5`. Node 22 plus exact pnpm
   10.27.0 accepted all 41 workspaces with `--frozen-lockfile` (exit 0),
   and an empty-store run explicitly said the lock was current before
   failing at `ERR_PNPM_NO_OFFLINE_TARBALL`. Nearby revision
   `d87981b99d2e7916905ac797fda79f33dc01190b` independently passes the
   same check. The earlier incoherence diagnosis was wrong; the current
   Directus wall is missing package metadata during offline deploy.
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
- **pnpm's own architecture** — the store is content-addressed underneath
  (`…/store/v*/files/` keyed by content hash), while a SQLite package
  index carries package-to-integrity metadata. pnpm supports `pnpm fetch`
  followed by `pnpm install --offline`, but the spike shows that this is
  not a bare-CAS protocol: the index is derived-looking yet required and
  is not regenerated from `files/` by pnpm 11.1.2 or 11.17.0.
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

1. **Diagnose the hang class first (no language change): complete.**
   The bounded verbose/strace/ss A/B identifies missing `cacert`, not
   IPv6 fallback. The follow-up is a migrate.md teaching line
   ("ecosystem fetches need `${pkgs.cacert}` imported — TLS-trust
   errors masquerade as hangs") plus a diagnostic: when a FETCH times
   out and the trace shows repeated TLS handshake failures, say so.
2. **Do not adopt bare-CAS store-as-artifact for pnpm.** The completed
   spike falsifies that mechanism at both required versions. Two
   independent FETCHes yielded identical CAS trees (Dozzle: 20,175
   files; Verdaccio: 56,280), but bare-CAS offline installs exited
   `ERR_PNPM_NO_OFFLINE_TARBALL`; pnpm did not reconstruct `index.db`.
   Independent indexes differed and embedded volatile `checkedAt`
   fields, so generic byte pinning or SQLite-dump normalization is not
   honest either. Existing FETCH/COPY/RUN features cannot express a
   cold-replayable pnpm artifact here. npm is different: npm 11.13.0
   replayed two network-silent `npm ci --offline --no-audit` installs
   from `_cacache/content-v2` alone, because package-lock integrity
   locates CAS content without `_cacache/index-v5`.

   **The surgical route exists upstream (web research, 2026-08-06):
   pnpm 11.7.0 shipped `frozenStore`** — first-class support for
   installing against READ-ONLY stores, with "Nix, bind mounts, OCI
   layers" as the documented use case. It opens `index.db` in SQLite
   immutable mode (no WAL/-shm sidecars) and "suppresses every code
   path that would write to the store"; the documented pairing is
   `--offline --frozen-lockfile` against a fully-populated store.
   That inverts our mechanism: do NOT strip or regenerate the index —
   **seal the whole store as fetched (files/ + index.db), a TOFU
   instance-pin**, and install frozen against it. The cross-fetch
   nondeterminism of `index.db` gets reclassified: it makes two
   fetches inequivalent as BYTES but not as STORES — replayability of
   one pinned instance is what cix needs, and frozenStore guarantees
   the install neither mutates nor regenerates anything (the
   consumed-volatility loop is cut by pnpm itself, not by us). cix's
   double-fetch probe should then require identity on `files/` and
   record index/metadata divergence as instance-volatility, without
   refusing the pin. Guards, all upstream-defined: pnpm ≥11.7.0 and
   Node ≥22.15/23.11/24 required (`ERR_PNPM_FROZEN_STORE_UNSUPPORTED_NODE`);
   stores must pre-contain build outputs for script-running packages
   (`ERR_PNPM_FROZEN_STORE_NEEDS_BUILD` fails upfront, we fetch
   `--ignore-scripts` anyway); incompatible with `--force`. The known
   offline-resolution footgun (pnpm#10715: `--offline` resolves semver
   against metadata including uncached versions) is sidestepped
   because `--frozen-lockfile` installs headless, without range
   resolution. Version reach: dozzle pins pnpm 11.17.0 (eligible);
   verdaccio 11.1.2 and directus 10.27.0 predate 11.7 — whether
   translation may upgrade the package manager past upstream's
   `packageManager` pin (recorded as a deviation in GAPS) is a taste
   call, not a mechanism gap.
3. **`WITH CACHE` lands as designed in nodes-and-edges** (recorded
   direction there): declared cache paths outside read-set evidence,
   persisted across builds. This addresses the scale face (exhibit
   5): module/package caches stop being sealed at all — the seal
   covers the project tree, not the ecosystem's cache. Also the
   honest answer for Go's module cache (filestash) without any Go
   special-casing.
4. **Keep frozen validation, but correct the Directus diagnosis.** A real
   manifest/lock mismatch should still be named as an upstream defect
   and must never trigger an automatic non-frozen install. Directus is
   not that case: both its pin and a nearby revision validate. Its
   missing offline deployment metadata belongs with exhibit 3's
   fetch-artifact problem, and it does not gate CIP-107's 14 narHash
   regenerations on finding another source revision.

Explicitly rejected: a global pnpm exception; weakening network
isolation; translating lockfiles by default (*2nix fallback stays a
fallback); pinning volatile trees as-is (a pin that cannot replay is
worse than no pin).

## 4. Open questions

- **Answered:** pnpm does not rebuild its store index from bare CAS at
  either corpus pin; npm can replay from content-v2 without index-v5
  when audit/fund network features are disabled.
- **Instance-pin semantics** (new, from the frozenStore route): does
  cix's pin model accept a TOFU instance-pin whose double-fetch probe
  shows index/metadata divergence — i.e. "identical `files/`, divergent
  derived index" graded as a valid pin rather than refused volatility?
  This is the one semantic amendment the surgical route needs from us;
  everything else is upstream mechanism.
- **Translation pnpm-version policy** — DECIDED (Mathijs, 2026-08-06):
  for the specific corpus targets, bump pnpm past 11.7 and RECORD it —
  the upgrade is an explicit, GAPS-visible deviation from upstream's
  `packageManager` pin, justified by upstream itself (pre-11.7 pnpm
  structurally cannot install from a read-only store).
- **Diagnostic + hint** — DECIDED (Mathijs, 2026-08-06): build the
  problem-class diagnostics that hint the user toward the solution —
  TLS-trust masquerade → "import `${pkgs.cacert}`"; offline-tarball /
  store-write walls → the frozenStore route with its version gates.
  Doc-anchor citations per D73, never CIP numbers.
- **Spike before adoption**: one verdaccio-shaped validation of the
  full route (seal files/+index.db as fetched → `frozen-store=true`
  install `--offline --frozen-lockfile` from the read-only pinned
  tree, twice, network-silent) with a pnpm ≥11.7 override, before any
  language/semantic change lands. (In flight as track/pnpm-frozenstore
  together with the two decided items above.)
- **WITH CACHE as the generic escape hatch + cache-path detection**
  (Mathijs's question, 2026-08-06; orchestrator's assessment delivered
  in chat): yes as escape hatch — WITH CACHE is the ecosystem-agnostic
  degradation (no per-PM knowledge, but no cold proof either; the
  frozenStore-class pinned-store route stays strictly stronger where
  an ecosystem offers it). Detection: surface, never auto-classify —
  candidate signals are (a) double-fetch probe divergence concentrated
  under one subtree, (b) the known cache-name/env vocabulary
  (XDG/*_CACHE/GOMODCACHE/npm_config_cache/cargo registry), (c)
  written-then-read-back subtrees outside the project tree. The
  diagnostic proposes `WITH CACHE <path>`; the author declares it
  (CIP-102: declared coarseness, never silent evidence exclusion).
  Awaiting Mathijs's read on the assessment before this leg is built.
- Where does the pinned store live in the artifact model — a FETCH
  output tree like today, or does `WITH CACHE` subsume it entirely
  (cache persisted, nothing pinned)? The difference is evidence:
  a pinned store replays cold; a cache does not. For pnpm, `files/`
  alone is not yet a valid pinned artifact, so `WITH CACHE` can improve
  a warm developer loop but cannot supply the missing cold proof.
- Seal-time engineering for large stores (exhibit 5) remains open: the
  CAS-only measurements establish determinism for 20k–56k-file trees,
  not whether parallel hashing or CAS-portion pinning makes sealing fast
  enough.
- The Homer/Dozzle cacert lesson generalizes into a prelude convention
  (migrate.md teaches `IMPORT ${pkgs.cacert}` for any networked
  FETCH) or a language affordance (FETCH implies cacert in its
  sandbox)? The latter touches determinism claims — needs its own
  taste call.

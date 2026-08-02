# Read-set step keying — early cutoff for builders

Status: **CIP-87, adopted 2026-08-02** (Mathijs; drafted the same day out
of the gitsitter compare Cixfile review). Decision in §5.

## 1. The problem

Builder step keys form a pure chain: every key hashes the directive, its
resolved arguments, COPY source hashes, and the **predecessor key** —
workspace bytes are never hashed. The consequence is that *ordering is
the cache*: authors must sequence `COPY Cargo.toml` + `COPY Cargo.lock`
→ `FETCH cargo vendor` → `COPY src/` → `RUN` or every source edit
changes the FETCH step's predecessor and re-runs the network step.
This is Docker's layer-ordering discipline, inherited wholesale — a
cognitive tax on every Cixfile author, and the single biggest source of
Cixfile verbosity in the gitsitter comparison (4 COPY lines + a split
vendor/build pair where the author's intent is "copy the source, vendor,
build").

With read-set keying the same Cixfile becomes:

```dockerfile
FROM github:NixOS/nixpkgs/<rev> AS pkgs
FROM github:mathijshenquet/gitsitter AS src

BUILDER build
  IMPORT ${pkgs.cargo} ${pkgs.rustc} ...
  COPY ${src}/ .
  FETCH cargo vendor --locked vendor > vendor-config.toml
  RUN cargo build --release --locked --offline
```

and a `src/main.rs` edit re-runs only the RUN step, because `cargo
vendor` never read `src/`.

## 2. Prior work

- **Docker** — the chain model we inherited; ordering-as-cache is its
  best practice and its best-known pain.
- **Nix derivations** — whole-input keying; same property (any input
  change rebuilds), mitigated by splitting derivations, which is the
  same ordering discipline at a different granularity (crane's
  `cargoArtifacts` split exists purely for this).
- **Bazel/Buck** — statically *declared* per-target inputs; precise but
  demands a build-graph author.
- **tup / Fabricate / Memoize** — traced read sets (FUSE, ptrace,
  LD_PRELOAD): the build tool observes what a command read and keys on
  exactly that. No declarations, no ordering discipline.
- **Shake / "Build Systems à la Carte" (Mokhov, Mitchell, Peyton
  Jones)** — names the ingredients precisely: *constructive traces*
  (store the read set + hashes with the result) and *early cutoff*
  (stop when the re-derived inputs match). This CIP is constructive
  traces for FETCH/RUN steps.
- **Cix itself, output side** — the lock memo already records only the
  paths downstream artifacts *consume*, and FETCH pins are
  consume-narrowed ("incidental cache files do not make a build flap").
  This CIP is the input-side dual of that decision: steps key on what
  they read, artifacts key on what they consume.

## 3. Recommendation

Trace the read set of every FETCH and RUN step inside the existing
sandbox, and make the step memo a **constructive trace**:

- **Static part** (as today, minus the predecessor key): directive text,
  resolved arguments, ENV as declared, ordered imports and offered
  closure, the versioned sandbox skeleton.
- **Dynamic part**: a map from each path the step *read* to its content
  hash — regular files by content, directory listings by entry-list
  hash (a readdir depends on the listing, not the entries' contents),
  and **negative lookups** recorded as nonexistence markers (a step
  that probed for `config.toml` and found nothing depends on it staying
  absent).

Memo lookup re-hashes exactly the recorded read set (mtime+size
fast-path before content hashing); if everything matches, the step is a
hit *regardless of what else changed in the workspace* — that is early
cutoff. On a miss the step runs and records a fresh trace.

Unchanged: COPY staging semantics (declared inputs staged fresh,
deletions included); the warm underlay and its path-dependence caveat
(workspace bytes still never enter keys); `--cold` as the audit that
proves builder reproducibility. The chain key survives only as the
address of the workspace lineage, not as the cache key.

Documentation flips the idiom: `COPY ${src}/ .` becomes the taught
default; manifest-first ordering is demoted to an optimization note for
pathological read sets.

## 4. Open questions

1. **Trace mechanism**: ptrace (complete, slow-ish, no kernel deps),
   fanotify (fast, misses negative lookups), FUSE indirection (complete,
   heavier plumbing). The FETCH probe machinery already snapshots — is
   it extensible, or does this want the bubblewrap boundary?
2. **Readdir granularity**: is entry-list hashing enough, or do glob
   patterns need prefix-set records?
3. **Trace multiplicity**: keep only the latest trace per step, or N
   recent traces (an A→B→A edit pattern then stays warm both ways)?
4. **Migration**: old chain-keyed memos are orphaned — fingerprint bump
   (the honest D48a precedent) or dual-read for one release?
5. **Hash cost**: a RUN like `cargo build` reads the whole tree — its
   lookup rehashes everything it read. Acceptable (it's what the key
   means), or does the mtime fast path need a per-builder stat cache?

## 5. Decision

Adopted as recommended. The open-question dispositions (Mathijs,
2026-08-02):

1. **Trace mechanism** — implementation detail, implementer's choice
   ("overweeg naar eigen inzicht"); pick for completeness first
   (negative lookups must be captured), speed second.
2. **Readdir granularity** — start with entry-list hashing;
   glob/prefix refinement is evidence-gated.
3. **Trace multiplicity** — start with a single latest trace (or
   unbounded if it falls out naturally); no fixed-N tuning.
4. **Doc idiom** — flips to copy-everything default; ordering demoted
   to an optimization note.
5. **Migration** — none; alpha. Old chain-keyed memos are orphaned via
   an honest fingerprint bump (D48a precedent).
6. **Hash cost** — mtime+size fast path is fine; further caching is an
   optimization detail, not design.

## Changelog

- 2026-08-02: drafted and adopted same day.
- 2026-08-02 (tracefast landing; Mathijs steered in-session): the warm
  gitsitter edit descended 84.83s → 8.31s WITH complete capture —
  green under the bar. Semantics amendment sanctioned live by Mathijs:
  RUN memos are VERIFYING-ONLY (a RUN that would have replayed
  re-executes — cheap in a warm workspace); FETCH memos stay
  constructive (pins and --cold replay require them). Also landed:
  mtime-preserving staging, trace-side hash reuse, per-tracee tracer
  parallelism, subprocess diet (9 calls, 0.28s — the measured libnix
  ROI is now marginal).
- 2026-08-02 (performance criterion, Mathijs): a warm one-line edit on
  the gitsitter compare must land around ~8–9s; above that the feature
  is ORANGE, and if it cannot beat crane's 16.46s the CIP itself is in
  question — early cutoff that loses the warm-edit race buys only UX.
  First post-landing measurement: 84.83s (complete ptrace capture +
  read hashing + delta storage on executed steps) against the old
  layer-split 7.46s. A trace-overhead reduction track owns this bar;
  any completeness-vs-speed trade that would relax §3's capture
  requirements is a finding for Mathijs, never a unilateral change.

## Regression surface (design, for the eventual track)

Assertions are **work-based, never wall-clock**: a machine-readable
`cix build --stats` (per step: executed | memo-hit, plus subprocess
count) is the channel. Hermetic mini-fixture in the cargo test tier — a
tiny two-file project whose FETCH pulls from a test-local HTTP server
(the tour/index tests already run those), so CI needs no real network:

- src-only edit → FETCH hit, RUN executed;
- manifest edit → FETCH executed;
- no-op → zero steps executed;
- normal / repeat / `--cold` converge byte-identically.

The gitsitter comparison stays `examples/compare/` + dated wall-clock
receipts in docs/nix-build.md; timing numbers are never CI-asserted.

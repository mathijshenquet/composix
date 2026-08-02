# track/readset — CIP-87: read-set step keying (early cutoff)

Read AGENTS.md first. Authoritative: docs/cips/0087-read-set-keying.md
(§3 recommendation + §5 Decision + regression surface). Prerequisite:
track/ergo landed the `--stats` channel and the hermetic mini-fixture —
extend both, do not fork them. This is the memo-core rework; you own
crates/cix-cixfile's builder engine for the duration. Work in
`.worktrees/readset` on branch `track/readset`. Keep
`crates/cix-cixfile/LOG.md` current.

1. **Trace capture**: record the read set of every FETCH and RUN step
   inside the existing sandbox. Mechanism is your choice (§5.1 —
   ptrace/fanotify/FUSE trade-offs are yours to weigh), with one hard
   requirement: completeness over speed — regular-file reads,
   directory listings, and NEGATIVE lookups (probed-and-absent paths)
   must all be captured. Record your choice and its failure modes in
   the LOG.
2. **Constructive-trace memo**: static part = directive text, resolved
   arguments, declared ENV, ordered imports + offered closure, sandbox
   skeleton version — NOT the predecessor key. Dynamic part = map of
   read path → content hash (files), entry-list hash (readdirs),
   nonexistence marker (negative lookups). Lookup rehashes exactly the
   recorded read set (mtime+size fast path allowed, §5.6); full match =
   hit regardless of other workspace changes; miss = run + record. One
   latest trace per step (§5.3; unbounded only if it falls out
   naturally).
3. **Unchanged semantics**: COPY staging (fresh, deletions included);
   warm underlay path-dependence; workspace bytes never in keys;
   `--cold` as the reproducibility audit — `--cold` must now also
   verify that replayed steps' recorded read sets match what the cold
   run actually read, and report divergence per step/line.
4. **Migration**: none (§5.5) — bump the builder fingerprint honestly;
   old memos orphan.
5. **Fixture + docs idiom flip** (§5.4): simplify
   `examples/compare/gitsitter/cix/Cixfile` to copy-everything
   (`COPY ${src}/ .` + FETCH vendor + RUN build); docs/cixfile.md
   teaches copy-everything as the default and demotes manifest-first
   ordering to an optimization note; docs/migrate.md's conversion
   guidance follows if it teaches the ordering idiom.
6. **Tests**: extend the hermetic mini-fixture with the CIP-87
   acceptance set — src-only edit → FETCH memo-hit + RUN executed;
   manifest edit → FETCH executed; no-op → zero steps; negative-lookup
   dependency (step probes an absent file; creating the file must
   re-run the step); readdir dependency (adding a file to a listed dir
   re-runs); normal/repeat/`--cold` byte-converge. All via `--stats`,
   never wall-clock.
7. **Receipts**: re-measure the gitsitter one-line-edit and no-op
   receipts in docs/nix-build.md (dated), showing the copy-everything
   Cixfile keeping FETCH warm across a src edit.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.

# Track: litdoc — literate documentation harness (docs that are tests)

Read `DESIGN.md` decision D19 first. Reference implementation of the pattern: gitsitter's
`tests/workflows.rs` → `docs/workflows.md` (a checkout is at
`/tmp/claude-1001/-home-mathijs-composix/c82a8a14-2a3d-4830-92d2-98baf3adb5ee/scratchpad/gitsitter`
— read `tests/workflows.rs` and `docs/workflows.md` to absorb the shape: scenario functions
build a markdown doc while asserting real behavior; an `--ignored` generator test writes the
file). Where ambiguous, choose the boring option and note it in your LOG — do NOT expand scope.

## Ground rules

- Work log: create and keep `crates/cix/LOG.md` current (timestamped, append-only).
- Territory: `crates/cix/tests/`, `docs/`, and dev-dependencies in `crates/cix/Cargo.toml`
  (+ workspace `Cargo.toml` dep entries if needed). Do NOT touch any `src/` of any crate, nor
  `crates/cix-index/`, `crates/cix-run/`, `DESIGN.md` (another agent works on cix-index in
  parallel).
- IMPORTANT scope restriction: scenarios cover ONLY local index flows — tag, ls, untag, GC
  roots, sidecars. NO `cix serve`, NO `cix pull`, NO claims (that surface is being changed in
  parallel; a publish/pull scenario comes later).
- Commit to your branch as you go. `cargo fmt --check`, `cargo clippy --workspace -- -D
  warnings`, `cargo test --workspace` green at the end.

## The harness — `crates/cix/tests/tour.rs`

1. Drive the real compiled binary: `env!("CARGO_BIN_EXE_cix")` (integration tests of a bin
   crate get this for free). Each scenario runs in an isolated environment: fresh temp dir under
   `target/test-tmp/` containing a private `CIX_STATE_DIR`. Real nix is available and required
   (`nix store add-path` for fixtures) — same assumption as the existing cix-index tests.
2. A small `Doc` builder that accumulates markdown: `heading`, `para`, and a `sh` transcript
   helper that (a) runs a command via the shell with the scenario env, (b) asserts exit success
   (or an expected failure), (c) appends a fenced block showing `$ <command as the reader would
   type it>` followed by actual captured stdout+stderr, normalized (below). Assertions about
   output content are made by the scenario code against the RAW output; the doc shows the
   NORMALIZED output.
3. Normalization (required for deterministic generation — apply to doc output only):
   - nix store hashes: `/nix/store/<32 base32 chars>-` → `/nix/store/…-`
   - unix timestamps (10-digit numbers in JSON `createdAt`) → `1700000000`
   - ages like `age=3s` → `age=0s`
   - absolute temp paths (the scenario's base dir) → `~`
   Determinism gate: the generator must produce byte-identical output across two consecutive
   runs in the same build (add a test that generates twice and compares).
4. Generation: `cargo test --test tour -- --ignored generate_tour` writes `docs/tour.md` with a
   header noting it is auto-generated, the command to regenerate, and version+commit (mirror
   gitsitter's header).
5. Drift check (NOT ignored, runs in plain `cargo test`): regenerate in memory, compare with the
   committed `docs/tour.md`; on mismatch fail with a message telling the developer to run the
   generation command. This is what makes docs-that-lie impossible.

## Scenarios (exactly these three, in this order)

1. **Tagging a build.** Narrative: nix produced a store path; give it a name. Create a small
   dir with a file, `nix store add-path` it, `cix tag <path> my-app:v1`, `cix ls -l`. Then SHOW
   the mechanism: `ls` the state dir's `roots/`, `readlink` the symlink, and `cat` the JSON
   sidecar — with prose explaining: the tag database is an ls-able symlink farm, and each
   symlink is a nix GC root (the pin IS the name).
2. **Moving a tag.** Tag a second store path as `my-app:v1` (retag), show `cix ls -l` reflects
   the new path, prose: tags are mutable pointers over immutable store paths; the old path is
   now unpinned (show the symlink now points at the new path).
3. **Untagging.** `cix untag my-app:v1`, `cix ls` (empty), prose: unpinned means the next
   `nix-collect-garbage` may reclaim it; nothing else in cix holds it.

Keep prose crisp and factual, in the voice of gitsitter's workflows.md. The doc's audience is a
developer evaluating composix in five minutes.

## Done criteria

fmt/clippy/tests green including the drift check and determinism test; `docs/tour.md` committed
and readable (render it mentally: headings, prose, transcripts); LOG.md final summary with any
deviations and open questions.

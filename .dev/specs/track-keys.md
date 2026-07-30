# track/keys — D56 EXPECT + D57 narrow read-keying (increment 1) + D58 CA trust

Read AGENTS.md first. Authoritative: docs/design.md **D56, D57, D58** (the five D57
invariants are law; mechanics are yours where the spec is silent). Context: D39/D40/
D48a for the model being superseded. Scope: crates/cix-cixfile (build engine, grammar,
lock), cix-run only if manifest surface moves (it should not), examples, docs
(cixfile.md), tour. Do NOT touch corpus/ (track/migrate-r4 runs there concurrently)
— corpus rewrites (adminer EXPECT, CACHE removals) happen in a follow-up corpus
round. Do not touch nix/scenarios/.

## D56 — EXPECT

- Grammar: `FETCH [<name>] EXPECT <sri-hash> <cmd…>` for both FETCH forms. With
  EXPECT: the fetch output must hash to the declared value — mismatch is a build
  error naming declared vs actual (like nix's fixed-output error); the lock records
  the declared hash (no TOFU). Without EXPECT: today's TOFU pinning unchanged.
- `--update-lock` on an EXPECT fetch is meaningless → clear error (change the
  EXPECT value instead).
- Tests: mismatch error, match success, lock behavior, both forms.

## D57 increment 1 — the keying rework

Implement to the invariants; concretely:

1. **Step keys become chain keys**: key(step) = hash(directive kind + command/args +
   offered closure set + key(previous step) + content hashes of any declared
   sources this step stages [COPY] + env). No workdir bytes in any key. FETCH steps
   keep their output pins (TOFU or EXPECT) as today — a FETCH's pin participates in
   its key.
2. **Consumed-path records replace snapshot plucks**: for every artifact-bound
   `COPY ${builder}/<path>` (statically known set), after the builder chain runs,
   add each consumed path to the store individually and record
   (chain key → path → content hash + store path) in the lock's memo section.
   Memo hit (chain key unchanged AND all needed paths recorded) → materialize from
   store without running. A newly referenced path not in the record forces a chain
   re-run. Whole-tree references (`COPY ${builder}/ .` or `${builder}` in later
   builders) consume the full left-behind tree: key on its content hash, record it
   as one tree object — legal, expensive, documented as such.
3. **Persistent overlay workspaces**: per (Cixfile path, builder name) a workspace
   survives builds. Staging materializes declared inputs fresh each run (exact:
   deletions propagate — implementation free to use overlayfs where cheap per D39,
   or fresh-dir + hardlink/copy with an upper-preserve rule; choose and document);
   everything the build WRITES persists for the next run. `cix build --no-cache`
   becomes `--cold` (alias kept, deprecation note): run with an empty upper.
   A wiped workspace must never change any result — prove it with a test (build,
   wipe, rebuild → identical item store paths).
4. **CACHE removed**: parse error with migration text ("CACHE was removed (D57):
   workspaces persist by default and nothing is keyed unless read; delete the
   line"). Remove cache-dir machinery. proj1 loses its CACHE line and its cp-dance:
   SERVICE COPYs move to `${build}/target/release/…` directly (the narrow-key
   showcase).
5. **Per-path attribution**: the cold-vs-warm sampling check (whatever verb/flag
   hosts it today plus the proj1 test) compares per consumed path and reports
   mismatches as "`COPY ${build}/<path>` (line N) differs between warm and cold" —
   the exact-line detector.
6. **Tour**: chapter on building with RUN + proj1 chapter rewritten to tell the new
   story: no CACHE line, direct narrow plucks, warm/cold equivalence shown via the
   existing marker trick (the marker file lives in the persistent upper now), and
   the `rm -rf workspace is always safe` property demonstrated. Prose quality per
   the tourbook bar.

## D58 — CA trust in FETCH

- FETCH sandbox env gains `SSL_CERT_FILE`, `GIT_SSL_CAINFO`, `CURL_CA_BUNDLE`
  pointing at the locked nixpkgs `cacert` bundle; cacert joins FETCH steps' offered
  closure automatically. Explicit env on the FETCH line overrides. RUN gets none of
  this (networkless). Update examples (whoami-style FETCHes go bare: PATH already
  applies inside builders — sweep full `${pkgs.…}/bin/` prefixes from in-builder
  FETCH/RUN lines in examples), docs/cixfile.md, and docs/migrate.md (two lessons:
  bare tools inside builders; no more SSL ceremony — but do NOT touch corpus/).
- Tests: a FETCH using bare git over https succeeds without any SSL env in the
  Cixfile; override still wins.

## Gate

`cargo fmt --all --check` · clippy `-D warnings` · `cargo test --workspace` ·
proj1 e2e: warm rebuild selective, `--cold` byte-identical, workspace-wipe test ·
tour regen + drift + determinism twice · `vm-dogfood` · `compose-fallback-vm` ·
`scenario-lifecycle` (tier guard). Exact repro commands in crates/cix-cixfile/LOG.md,
append-only. This is engine surgery: commit in small reviewable steps.

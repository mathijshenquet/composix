# track/leaks — probe tempdir self-clean + harness TMPDIR

Read AGENTS.md first. Two ops nits from the 2026-08-01 verification round
(.dev/LOG.md top entry). Behavior-preserving: no measurement re-runs, no
number changes in docs. Work in `.worktrees/leaks` on branch `track/leaks`.
Keep `crates/cix-build/LOG.md` current (tracked, append-only).

## 1. FETCH probe leaks `/tmp/cix-fetch-probe-*` dirs

Observed: after update-lock probe runs (D69b), `cix-fetch-probe-*` dirs
persist in the temp root — large trees (node_modules-class), twice a factor
in tmpfs inode exhaustion. They must self-clean on success AND failure.

- `copied_snapshot()` in `crates/cix-build/src/build_chain.rs` (~line 1056)
  returns a `tempfile::TempDir`, which should Drop-clean — so first
  REPRODUCE and root-cause the escape. Hypotheses to check, in order:
  (a) read-only dirs/files copied by `copy_tree` (npm/pnpm trees carry r-x
  dirs) make `remove_dir_all` fail, and TempDir's Drop swallows the error;
  (b) an early `std::process::exit` path skips Drop; (c) the TempDir is
  kept/leaked somewhere downstream. Record the actual mechanism in the LOG.
- Fix accordingly: explicit cleanup with error propagation (e.g.
  `TempDir::close()`), permission-fixing removal for read-only trees, and
  cleanup that also runs on the probe's error paths.
- Test: after a probe run over a fixture tree that includes a read-only
  subdir, the temp root contains zero `cix-fetch-probe-*` entries; same
  after a probe that FAILS partway. Respect TMPDIR in the test (tempfile
  already does).

## 2. measure-warm.sh hardcodes /tmp

`examples/compare/gitsitter/measure-warm.sh:9` uses
`mktemp -d /tmp/…` — honor TMPDIR: `mktemp -d "${TMPDIR:-/tmp}/…"`.
Sweep `examples/compare/` for any other hardcoded `/tmp` and fix the same
way. Do NOT re-run the measurement or touch docs/nix-build.md numbers.

## Gate

fmt / warning-denied clippy / workspace tests / the new probe-cleanup tests /
full `devenv shell -- nix flake check -L` (the FULL tier — no cherry-picked
subset, per AGENTS.md). Exact repro commands in crates/cix-build/LOG.md.
Commit on this branch when green.

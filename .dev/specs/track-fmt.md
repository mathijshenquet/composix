# track/fmt — D74: `cix fmt`

Read AGENTS.md first. Authoritative: design.md **D74** (canon v1, CLI shape)
and **D53** (comments preserved verbatim). Work in `.worktrees/fmt` on branch
`track/fmt`. Keep `crates/cix-cixfile/LOG.md` current (tracked, append-only).

## Architecture

1. The existing parser (`crates/cix-cixfile/src/parser/`) is line-based and
   LOSSY — it assembles a semantic `Cixfile` and discards trivia. Do NOT
   retrofit trivia into it. Build a separate lossless document model in a new
   `fmt` module of cix-cixfile: a line-level scanner that classifies each
   physical line (blank / comment / top-level directive / block header
   (BUILDER, SERVICE, ARTIFACT, …) / block-body directive / heredoc open
   line / heredoc body / heredoc terminator / `\`-continuation line) and
   keeps the original text. Derive the classification rules from
   `parser/machine.rs` + `parser/directives.rs` — the scanner must agree with
   the real parser about what a heredoc is and where blocks start.
2. Formatting is parse-gated (D74): run the REAL parser first; on ParseError
   print the ordinary parse error, write nothing, exit non-zero. Never emit
   output for input the parser rejects.
3. Two invariants, enforced in code and tests:
   - **Semantic preservation**: `parse(format(x)) == parse(x)` (derive
     Eq/PartialEq on the parse result if missing).
   - **Idempotence**: `format(format(x)) == format(x)`.

## Canon v1 (D74 — deliberately minimal)

- Block bodies indented two spaces; top-level directives (prelude FROMs,
  top-level FETCH, block headers) unindented. NOTE: today's repo Cixfiles
  (e.g. `examples/build/proj1/Cixfile`) have UNINDENTED bodies — the canon
  reindents them; that is intended.
- Exactly one blank line between blocks; prelude at top, then blank line,
  then blocks. No leading/trailing blank lines; file ends with one `\n`.
- Single spaces between tokens and around `=` (e.g. `PORT http = 18084`).
- Author line breaks are preserved, including `\` continuations; a
  continuation line is indented 4 spaces past its directive's indent.
- Trailing whitespace stripped; CRLF → LF.
- Comment lines reproduced byte-for-byte (modulo CRLF strip) at their
  position, per D53. Heredoc open line gets normal directive treatment;
  heredoc BODY and terminator line byte-for-byte untouched.
- Nothing else: no alignment, no sorting, no line-length wrapping (v2,
  evidence-gated). When canon is silent, preserve the input.

## CLI (D74 — no other flags)

- `cix fmt [PATH…]`, wired as a subcommand in `cix_cixfile::cli::Command`
  (flattened into the `cix` binary, see `crates/cix/src/main.rs`).
- Default `.`; directory args are searched recursively for files named
  exactly `Cixfile`, `.gitignore`-respecting (use the `ignore` crate);
  explicit file args are formatted regardless of name.
- Apply-in-place by default; only write when content changed.
- `--check`: no writes; print a per-file unified diff (the `similar` crate
  or a minimal hand-rolled unified diff — smallest footprint wins); exit 1
  if any file would change, 0 otherwise.
- `-`: stdin → stdout (for editors); with `--check`, diff to stdout + exit
  code. `-` mixed with other paths is an error.

## Tests

- Golden fixtures under `crates/cix-cixfile/tests/` (own subdir, NOT under
  `torture/`): messy input → canonical output snapshots covering every canon
  rule, comments in all positions, heredocs, continuations, CRLF, tab
  indentation, `=` spacing.
- Torture sweep: for each fixture in `tests/torture/`, if it parses then
  fmt succeeds, is idempotent, and is semantic-preserving; if it does not
  parse, fmt fails with the same parse error and writes nothing.
- CLI-level: --check diff + exit codes; stdin mode; .gitignore respected;
  unchanged files not rewritten (mtime).

## Repo adoption + gate wiring

- Run `cix fmt` over the repo's real Cixfiles (`examples/`) as its OWN
  commit ("apply fmt canon"), separate from the implementation commits, so
  review stays possible. Regenerate the tour if it embeds Cixfile text.
- Lock stability check: after reformatting, a clean `update-locks` run on a
  reformatted example must produce a byte-identical `Cixfile.lock`. If it
  does not, STOP and report — that is a keying leak to surface, not paper
  over.
- `cix fmt --check examples` joins the standard gate: add it next to the
  existing fmt/clippy gate steps (devenv scripts / flake checks — wherever
  the current gate lives). Scope is `examples/` — test fixtures
  (torture, golden inputs) are deliberately non-canonical and stay out.
- Document `cix fmt` in docs/cixfile.md (short section, D74's words). No
  D-numbers in user-facing text (D73 addendum) — doc anchors only.

## Gate

fmt / warning-denied clippy / workspace tests / tour regen + drift /
`cix fmt --check examples` / full `devenv shell -- nix flake check -L`
(the FULL tier — no cherry-picked subset, per AGENTS.md). Exact repro
commands in crates/cix-cixfile/LOG.md. Commit on this branch when green.

## Addendum (2026-08-01): fix the surfaced COPY keying leak, then finish

The lock-stability stop was correct — thank you. Verdict: the leak is a
bug against decided keying semantics (design.md D48a: memo keys derive
from INPUTS; D69 consumed-set keying), not a design question. Fix it on
this branch, then complete the halted adoption. Steps:

1. `git fetch origin && git merge origin/main` first (main has moved:
   vm-dogfood fix, possibly track/leaks which also edits build_chain.rs).
2. Root-cause the exact changed key bytes: dump the step-key material for
   original vs reformatted proj1 and record in the LOG which component
   differed. (Your LOG's semantic-equality proof and the changed memo key
   look contradictory if `copy.source` is the parsed field — pin down
   what physical text actually reaches the key.)
3. Fix so step keys derive ONLY from parsed semantics + content hashes:
   for COPY that means the semantically-relevant parsed fields (dst, and
   source only insofar as its meaning — content is already covered by the
   declared-sources nar hash; physical spelling/whitespace must never
   reach the key). Changed key derivation orphans old memo entries — one
   cold rebuild — acceptable in alpha (D72), but say so in the LOG.
4. Regression test: formatting a fixture Cixfile leaves its builder step
   keys (and a clean update-lock's Cixfile.lock) byte-identical.
5. Re-prove the D69 pinkeys acceptance still holds (byte-identical locks
   across a workspace wipe between clean update-locks).
6. Resume the halted plan: examples reformat as its own commit, tour
   regen, `cix fmt --check examples` gate wiring, docs. Then the full
   gate from the section above.

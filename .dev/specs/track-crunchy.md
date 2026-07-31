# track/crunchy — adversarial human Cixfiles: error quality + honest forgiveness

Read AGENTS.md first. Mathijs: write adversarial, crunchy, human-like Cixfile
syntax; make the error messages good and the language forgiving. Scope:
crates/cix-cixfile (parser diagnostics), a torture-fixture corpus under
crates/cix-cixfile/tests/, docs touch-ups only where a message references
docs. Runs parallel to track/pinkeys (build_chain — do not touch it) and
track/cigreen2 (cix-index/scenarios — do not touch those).

## The boundary, stated up front
Forgiving ≠ guessing. NEVER silently accept ambiguity or infer intent —
that violates the minimal-magic budget. Forgiveness means: (a) accept
meaning-preserving syntax variance; (b) reject everything else with a
message that names the problem, the location, and the likely fix.

## Work
1. **Torture corpus**: write ~40–60 small Cixfiles the way real humans mangle
   them: typo'd directives (SERVIC, IMPROT, EXPOSED), docker idioms pasted
   verbatim (FROM ubuntu:22.04, RUN apt-get install, WORKDIR, CMD,
   ENTRYPOINT, COPY --from=build), wrong argument order, missing AS, case
   variants (from/From/FROM), tabs, CRLF, trailing whitespace, weird
   continuation placement, quoting mistakes (unterminated, smart quotes ""),
   stray `=`/`:`, comments in odd positions, empty blocks, duplicate
   binders, `${pkg.x}` vs `${pkgs.x}`, attrpath typos, heredoc mistakes.
2. **Snapshot + grade every message**: a test harness that runs each fixture
   and snapshots the diagnostic. Grade against: names the problem? exact
   line? suggests the fix? cites the D-number when it is a migration? Keep
   the snapshots as committed tests so message quality is drift-checked
   forever.
3. **Improve the poor ones**: suggestions via edit-distance over the live
   directive table ("unknown directive SERVIC; did you mean SERVICE?");
   docker-idiom directives get purpose-built messages pointing at the
   migrate.md mapping (WORKDIR/CMD/ENTRYPOINT/EXPOSE/USER etc. each get the
   honest "this is docker vocabulary; cix spells it …" hint); attrpath and
   binder typos suggest near-matches among bound names.
4. **Honest forgiveness**: tolerate meaning-preserving variance — amount of
   whitespace, blank lines, trailing whitespace, CRLF line endings,
   indentation depth. Directive keywords stay case-SENSITIVE (lowercase
   `from` = error with a "directives are uppercase" hint, not silent
   acceptance — the file is a document, canon matters; cix fmt will own
   canon).
5. Record per-fixture before/after in the LOG; keep messages terse — no
   essays, one problem + one fix per diagnostic.

## Gate
fmt / warning-denied clippy / workspace tests (incl. the new snapshot suite)
/ tour regen + drift + determinism twice / vm-dogfood. Exact repros in
crates/cix-cixfile/LOG.md. Commit on this branch when green.

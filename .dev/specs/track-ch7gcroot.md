# track/ch7gcroot — ch7 dev-loop gcroot cleanup fails on CI (drift leak #3)

CI run (diagnostics now print drift diffs) names it exactly: in
docs/tour/07-dev-loop-docker.md the CI render contains
`/usr/bin/rm: cannot remove '/run/user/1001/cix/gcroots/cix-run-faithful-NONCE.service.root': Permission denied`
which the committed (beast) doc lacks, shifting subsequent lines (the
elided-warnings placeholder lands in a different position as a
consequence). Two prior leaks in this saga were fixed causally
(ephemeral probe path; granular-warning elision) — do the same here.

Investigate FIRST: why does removing the gcroot fail on the GitHub
runner but not on beast? (Ownership/sticky-bit of
/run/user/<uid>/cix/gcroots entries after a --user run under the
runner environment; reproduce reasoning from the chapter's command
sequence in crates/cix/tests/tour_scenarios/dev_loop.rs.) Then fix at
the right layer:
- If the DOCUMENTED cleanup flow is wrong/fragile (e.g. the chapter
  removes a root the runtime should own), fix the chapter's commands
  or the product's gcroot lifecycle so the documented flow works on
  both host classes — preferred.
- Only if the failure is genuinely host-environmental AND the flow is
  correct, normalize/elide with a tight pattern and a comment naming
  this run.
The line-ordering shift is a symptom; do not chase it separately.

Discipline: branch `track/ch7gcroot` from current main, LOG
`crates/cix/LOG.md`; full agent tier (fmt included — a micro-round
was lost to fmt yesterday), value-checked captures; the fix must keep
the local tour suite green AND be argued for the CI environment
(old-manager runner, /run/user semantics). Clean branch; do not
merge.

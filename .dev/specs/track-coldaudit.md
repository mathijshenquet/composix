# track/coldaudit — determinism sampling in the test suite (D47e made real)

Read AGENTS.md first. Doctrine: D47(e) prescribes "sampled clean rebuilds as
the bridge check"; nothing runs it routinely today. Scope: crates/cix (a new
test target) + gate-convention docs. Do NOT touch corpus content or cix-cixfile
sources.

1. New ignored cargo test target `cold_audit` (crates/cix/tests/cold_audit.rs,
   tour-harness style): enumerate every examples/**/Cixfile; for each, run the
   real cix binary: `cix build <dir>` (warm) then `cix build --cold <dir>`;
   assert the JSON member maps are identical (content-addressed paths ⇒ path
   equality IS byte equality). Needs network for FETCH examples ⇒ `--ignored`,
   run host-side like the tour generator. Clear per-example failure output
   naming the diverging member and both paths.
2. Corpus sampling mode: env `COLD_AUDIT=<corpus-pair>` runs one
   corpus/migrate pair the same way (fetch context first via fetch.sh) —
   manual/periodic use, not part of the default sweep.
3. Wire it into the prescribed gate: document in the test file header and add
   the invocation (`devenv shell -- cargo test -p cix --test cold_audit --
   --ignored`) to the gate lists in .dev/specs/README-gates note if one
   exists; otherwise state in the LOG that future track specs must include it
   (the orchestrator will carry it into new specs).
4. Prove it catches: a temporary negative test (or documented manual repro) in
   which a deliberately nondeterministic builder (e.g. `RUN date > x`) yields
   a cold mismatch and the audit fails with the intended message. Do not
   leave the nondeterministic fixture in the default sweep.
5. Gate: fmt / warning-denied clippy / workspace tests / the new cold_audit
   run itself green over all examples / tour drift (expected untouched) /
   vm-dogfood. Exact repros in crates/cix/LOG.md (create if absent, tracked).
   Commit on this branch when green.

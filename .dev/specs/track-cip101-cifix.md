# track/cip101-cifix — sigterm scratch test still red on CI (round 2)

CI run 31046375147: `sigterm_removes_live_build_scratch` still fails
("cix did not create scratch under $TMPDIR/temp") while the two
sibling tests pass — the liveness guard landed, the TEST is still
racy on slow hosts. Prime suspect: intra-binary parallelism — cargo
runs the three fetch_probe_cleanup tests in parallel threads; on a
slow runner the startup-sweep test's cix invocation can interleave
with the sigterm test's pre-lock window, or the wait budget simply
starves under cold-cache CI timing. Beast is too fast to show it:
REPRODUCE FIRST under constrained resources (e.g. `taskset -c 0
nice -n 19 cargo test -p cix --test fetch_probe_cleanup` in a loop,
or a cold TMPDIR + cpu-limited run) until you see the CI failure
locally; only then fix.

Fix requirements: deterministic, not budget-inflation — serialize the
binary's tests (shared mutex like the tour harness, or
`--test-threads=1` enforced via harness), and/or an explicit sync
point (cix signals scratch creation; the test waits on the signal,
not on polling with a deadline). The reproduction loop must then pass
N=20 consecutive constrained runs, captured.

Discipline: branch `track/cip101-cifix` from current main, LOG
`crates/cix-build/LOG.md`; agent tier: cargo suite (the affected
binary constrained AND unconstrained) + fmt/clippy; the VM matrix is
untouched by a test-only change — selector prices it. Value-checked
captures. Clean branch; do not merge.

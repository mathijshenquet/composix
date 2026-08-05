# track/tourdet — tour render nondeterminism on slow hosts (main-CI red, round 3)

CI run 31049255420: `generated_tour_is_deterministic` and
`tour_matches_committed_document` fail — two consecutive renders on
the SAME runner differ (assert at crates/cix/tests/tour.rs:1069; GH
strips the diff payload). This is almost certainly the known
host-only flake two tracks logged today as "second web run printed
its unit" (cip101's gate round; cip105's tour receipts) — real
nondeterminism in tour generation that beast's speed masks: the tour
executes real commands, and under slow timing a second service run's
output (unit lines, ordering, readiness residue) leaks into the
rendered document.

Method (the cip101-cifix recipe, proven today):
1. REPRODUCE FIRST under constraint: loop the two tour tests under
   `taskset -c 0 nice -n 19` (plus a cpu-quota scope if needed) until
   the assertion fires locally; capture BOTH renders and diff them —
   the diff names the leaking content. Do not fix before you can
   print that diff.
2. Fix at the cause: whatever host-timing-dependent content leaks
   (ambient unit listings, second-run output, readiness timing text)
   gets either properly serialized/awaited in the harness or
   deterministically normalized at the leak site — cip106's
   normalization harness is the home. No blanket normalization that
   would hide REAL drift; the committed tour must still catch content
   changes.
3. Prove: N=20 constrained loops of both tests green, plus the
   ordinary unconstrained suite, captured value-checked.

Discipline: branch `track/tourdet` from current main, LOG
`crates/cix/LOG.md`; agent tier: cargo suite + fmt/clippy + tour
regen/drift; VM selector prices the diff. Clean branch; do not merge.

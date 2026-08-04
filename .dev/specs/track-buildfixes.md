# track/buildfixes — build-side defect trio (EXPECT warm validation, bare error, warm-root COPY)

Read AGENTS.md first (gate convention; synchronous receipts). Work in
the herdr worktree on branch `track/buildfixes`. Keep
`crates/cix-build/LOG.md` current (dated heading; commit it). All three
items live in docs/open-questions.md "Open for agents" with verified
evidence; the traefik corpus case deliberately preserves the EXPECT
repro (corpus/migrate/traefik — read its GAPS.md; do NOT edit corpus
files, the repro stays until this fix merges and a later corpus round
consumes it).

## 1. EXPECT not validated against the recorded pin on warm builds

A wrong/copy-pasted EXPECT builds green while the fetch memo-hits; the
mismatch only fires on live refetch. Also observed: the lock recorded
the identical narHash for two different fetches' pins while stepMemos
showed different content — root-cause that recording path too. Fix: at
plan time, string-compare each declared EXPECT against its recorded
lock pin and fail with a spanned error naming both values on
divergence; add the regression covering the traefik shape (two fetches,
one wrong EXPECT, warm memo present). Reproduce first from the traefik
case in a scratch copy.

## 2. Bare `Error: Not a directory` (fhsspike, the real directus blocker)

A failing build op surfaced a context-free io error — no path, no step.
Reproduce from the track/fhsspike branch's directus Cixfile (branch
exists; fetch context via corpus/migrate/fetch.sh directus in a scratch
checkout — do not touch the corpus tree). Root-cause, then give the
error its path + step attribution (D73 spirit: every io error a cix
operation makes carries what it was doing to which path). If the
root cause is a genuine product bug beyond diagnosability, fix it if
narrowly scoped or STOP-report if it needs design.

## 3. Warm-root duplicate COPY rejection (watchtower finding)

After a Cixfile edit, re-running against a warm builder workspace
rejected "direct duplicate COPYs because the warm root is already
populated" — luna contorted a translation around it before correction.
Reproduce per the NOTES in the watchtower corpus case's receipt/GAPS
(the exact steps are recorded). Expected semantics: a plan change
against a warm workspace reconciles or invalidates the workspace —
deleting a workspace is always correct, so silent invalidation is
acceptable; erroring on the re-run is not. Fix + regression in the
warm-workspace test tier.

FENCE: track/runfixes owns crates/cix-run; track/corpusk8s owns
corpus/ + the scenario roster; track/tourpolish owns the tour harness.
Your domain: crates/cix-build, crates/cix-cixfile if plan-time
validation lives there, its tests, open-questions updates, your LOG.

## Gate

Standard agent tier + focused scenarios you touch (df-guard; bounded).
Synchronous receipts.

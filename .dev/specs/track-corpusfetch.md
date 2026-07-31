# track/corpusfetch — stop vendoring upstream sources in corpus/migrate

Read AGENTS.md first. Mathijs (2026-07-31): upstream source trees must not live
in our repo — replace with a pinned download script and gitignore the fetched
content. Scope: corpus/migrate only; NO crate changes.

## Current state
Each corpus/migrate/<name>/ has exactly six files of ours (Cixfile,
Cixfile.lock, Dockerfile, SOURCE, check.sh, receipt.md); everything else is a
vendored upstream tree under <name>/context/ (~75MB, ~2700 files; dozzle,
watchtower, verdaccio are the bulk). SOURCE records the upstream git URL and a
"Resolved revision" sha.

## Work
1. `corpus/migrate/fetch.sh` (one shared script, bash): `fetch.sh <name>` and
   `fetch.sh --all`. Parses <name>/SOURCE for the repo URL + resolved revision,
   clones to a temp dir, checks out exactly that revision, and installs the
   tree at <name>/context/ (no .git inside; idempotent — safe to re-run;
   removes a pre-existing context/ first). Keep it boring: plain git clone +
   checkout, clear errors when SOURCE lacks a parseable URL/rev.
2. VERIFY BEFORE DELETING: for every <name> that has a vendored context/, run
   the fetch into a scratch location and `diff -r` it against the vendored
   tree (excluding any .git). Byte-identical ⇒ proceed. Any mismatch ⇒ STOP
   for that candidate, record the honest finding in its receipt.md and the
   corpus LOG, and fix SOURCE's pin so it does reproduce (the receipts' claims
   are tied to the vendored content — the pin must reproduce it exactly).
   Dockerfile stays tracked (it is the tiny migration input artifact, readable
   without fetching); only context/ trees leave the repo.
3. `.gitignore`: add `corpus/migrate/*/context/`. Then `git rm -r --cached`
   the vendored trees. Prove with `git check-ignore corpus/migrate/echo-server/context/README.md`
   (after a fetch) and a clean `git status`.
4. check.sh in each candidate that needs context/: add a guard that exits with
   "context/ missing — run ../fetch.sh <name> first" when absent. No other
   check.sh behavior changes.
5. Docs: note the fetch step wherever the corpus workflow is described
   (corpus/migrate/CANDIDATES.md curation notes and/or the corpus LOG), so a
   fresh clone knows `./fetch.sh --all` precedes any check.sh run.
6. Smoke: after the removal, on a fresh `fetch.sh echo-server`, run
   `corpus/migrate/echo-server/check.sh cix` (build cix first:
   `devenv shell -- cargo build`) and record the result. Do not run the full
   corpus matrix.

## Gate
No crate sources touched (assert with `git diff --stat`); fetch-verification
diffs recorded per candidate; `git status` clean modulo intended changes.
Exact repro commands + per-candidate verification results appended to
corpus/migrate/LOG.md (it exists). Commit on this branch when green.
Note: git history still contains the old trees; history rewrite is explicitly
out of scope.

# track/expand-postgres-registry — two new corpus cases (state-heavy band)

Add corpus/migrate/docker/postgres and corpus/migrate/docker/registry,
following the existing case anatomy exactly (read docs/corpus.md "How
this corpus is maintained" and copy the structure of a recent green
case such as mailpit or ntfy): Dockerfile (upstream's, faithful),
SOURCE (provenance + chosen tag), check.sh (runtime probe contract),
Cixfile + Cixfile.lock, receipt.md, GAPS.md, context via
corpus/migrate/fetch.sh.

Candidates (from corpus/migrate/CANDIDATES.md, verified rows):
- postgres: docker-library/postgres 17 trixie Dockerfile (PGDG apt
  vendor repo, locale gen, gosu, initdb entrypoint, VOLUME). Probe:
  `pg_isready`. State-heavy: exercises state roles/initdb lifecycle —
  grade any state-role friction into GAPS with the right arrow (the
  arbitrary-path realization defect is KNOWN and in repair on
  track/staterole-bindfix; cite it rather than re-diagnosing).
- registry: distribution/distribution main Dockerfile (multi-stage Go
  build via tonistiigi/xx cross tooling — the faithful translation
  targets a NATIVE build; record the cross-compile dissolution
  honestly). Config YAML, VOLUME /var/lib/registry. Probe: GET /v2/
  returns 200 {}.

Both upstreams pull from vendor repos/network at build: prestage as
explicit pinned FETCHes — no install-time network. Unpinnable or
drifting artifacts are honest walls: record in GAPS.md with the exact
failing form and keep whatever partial case is real, graded honestly.

Per case: faithful build receipt (synchronous exit 0), runtime probe
via check.sh where achievable, dissolved twin per the standard
contract, honest GAPS.md (deviation → routing arrows), corpus.md rows
with desk-vs-verified grades distinguished, cold-stage compatibility
(corpus/migrate/regen-stage.sh conventions: upstream-* files,
context/).

Discipline: branch `track/expand-postgres-registry` from current main,
LOG `crates/cix/LOG.md` (append, timestamped, FRICTION section —
record every language form you reached for that did not exist or
misbehaved). Gates: corpus receipts + corpus suite; `cargo test
--workspace` once only if Rust is touched (expected: none); no VM
matrix for corpus-only work. Synchronous value-checked receipts only.
Clean committed branch; do not merge.

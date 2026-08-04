# track/corpusk8s — corpus/migrate/{docker,k8s}/ restructure + k8s skeleton

Read AGENTS.md first (gate convention; synchronous receipts), then
docs/corpus.md §"How this corpus is maintained". Work in the herdr
worktree on branch `track/corpusk8s`. Keep `corpus/migrate/LOG.md`
current (dated track heading; commit it).

Mathijs's direction (2026-08-04): the corpus grows a k8s axis, so the
existing Docker cases move under `corpus/migrate/docker/` and
`corpus/migrate/k8s/` is born. This track is the RESTRUCTURE plus the
k8s skeleton — no k8s conversions yet (those are a later luna wave).

## 1. The move

- `corpus/migrate/<case>/` → `corpus/migrate/docker/<case>/` for all 21
  cases, `git mv` so history follows. `CANDIDATES.md`, `fetch.sh`,
  `regen-stage.sh`, `LOG.md` stay at `corpus/migrate/` (shared).
- Update every path-bearing consumer, verified by running it:
  - `corpus/migrate/fetch.sh` and `regen-stage.sh` (case dirs one level
    deeper; keep their CLI as `<case>` for docker cases, add an
    optional `docker/<case>`-style qualified form that k8s cases will
    use later).
  - Every case `check.sh` (relative `../../../target/debug/cix` and
    `../fetch.sh` references gain one level).
  - The corpus browser generator (`crates/cix/tests/corpus.rs`):
    discover cases under `docker/` and `k8s/`, render the axis in the
    index (two sections), keep per-case page filenames unique
    (`docker-<case>.html` / `k8s-<case>.html` or subdirs — your call,
    recorded; fix inbound links in docs/corpus.md).
  - The CIP-84 closed-root audit roster (exhaustive-directory property
    must keep holding — a new axis dir cannot silently escape
    classification).
  - The AGENTS.md ledger-currency grep pattern
    (`corpus/migrate/*/GAPS.md` → cover both axes).
  - Any other `corpus/migrate/` path literal in docs/, scripts, tests
    (grep exhaustively; list what you touched in the LOG).
- ~/CLEANUP.md and open-questions references to case paths: leave (they
  are historical prose).

## 2. The k8s skeleton

- `corpus/migrate/k8s/CANDIDATES.md`: seed from docs/corpus.md's k8s
  survey rows — at least: kubernetes-examples guestbook
  (Deployment+Service pair), a CronJob case (Airflow migrate Job row or
  a cleaner canonical CronJob), node-exporter DaemonSet (likely an
  honest refusal analysis — host-level agent), ingress-nginx
  (controller — likely refusal/partial), plus 1–2 you judge canonical
  (StatefulSet-shaped is interesting for STATEDIR mapping). One line
  each: source URL, what it exercises, expected verdict shape.
- A `corpus/migrate/k8s/README.md` stub: what a k8s case directory will
  contain (manifests in place of Dockerfile; same GAPS/receipt/check
  conventions; conversions target Cixfile+compose.json).
- Do NOT write conversions or a k8s teaching prompt yet — record in the
  LOG that docs/migrate.md's scope (Dockerfile→Cixfile) will need a
  k8s sibling or section when the wave starts.

## Gate

Standard agent tier + corpus browser regen/drift + the focused
closed-root audit scenario (the roster change is load-bearing).
df-guard before VM work. Bounded. Synchronous receipts.

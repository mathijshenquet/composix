# track/hooks — migrate-on-upgrade lifecycle hooks (corpus demand #5)

STATUS: design-position spec. Hard choices pending Mathijs (⚖); depends on D47 APP.
Corpus evidence: helm hook-Jobs (post-install/post-upgrade), Airflow's
wait-for-migrations initContainer — every framework with a schema has this need;
docker compose has NO answer (command-chain hacks like Plausible's).

## Design position

Compose composite-level `"hooks": { "pre-switch": ["<app-child-name>", …] }` — the
named APPs run **between generation build and unit switch** during `cix up`:
new-generation code, run-to-completion, in the composite's context (edges to the OLD
still-running db — exactly the rails/django/airflow migration pattern). All hooks
pass → switch + restart-changed proceeds; any hook fails → **the switch does not
happen**, old generation keeps running untouched, `cix up` reports the hook's output.

## ⚖ Hard choices (Mathijs)

- **Failure leaves half-run migrations.** The abort keeps UNITS consistent (old
  generation still runs) but the DATABASE may hold a half-applied migration — that is
  the app's transactional responsibility, not cix's. Options: (a) state exactly that,
  loudly, in docs (helm/k8s do the same silently), (b) add a `post-rollback` hook so
  apps can attempt compensation. Recommendation: (a) for v0; (b) only ever
  evidence-gated.
- **Hook phases**: v0 = `pre-switch` only? `post-switch` (smoke test after restart,
  failure → auto-rollback!) is tempting but auto-rollback on hook failure is a big
  semantic step (it makes hooks load-bearing for availability). Recommendation:
  pre-switch only in v0; post-switch+auto-rollback is its own design round with the
  scenario tier as evidence.
- **Timeout**: hooks must be bounded (a hung migration blocks up forever). Default
  10min, per-hook override? Recommendation: yes, with the timeout in the up output.

## Scope & gate

cix-compose (schema + up sequencing); scenario: upgrade with passing hook applies;
upgrade with failing hook leaves old generation serving and exits nonzero; hook runs
against old db via the edge (assert observable order).

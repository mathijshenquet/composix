# track/hooks — migrate-on-upgrade lifecycle hooks (corpus demand #5)

STATUS: design-position spec. Hard choices pending Mathijs (⚖); depends on D47 APP.
Corpus evidence: helm hook-Jobs (post-install/post-upgrade), Airflow's
wait-for-migrations initContainer — every framework with a schema has this need;
docker compose has NO answer (command-chain hacks like Plausible's).

## Resolved (D48f — Mathijs's round: "systemd heeft al hooks toch?")

The track SHRINKS: v0 builds no hook machinery at all.

- **The systemd-native shape covers the demand**: a migration is an ordinary oneshot
  APP unit in the composite (`RemainAfterExit=yes`), with the app `After=` +
  `Requires=` it. Content-addressing supplies run-on-upgrade for free: the oneshot's
  `ExecStart` contains its item store path, so restart-changed re-runs it exactly
  when its app version changed. Chain-style cases (Plausible's `createdb && migrate
  && run`) are `ExecStartPre`, also native.
- v0 work is therefore only: make the oneshot-barrier pattern EXPRESSIBLE in compose
  (an APP child that services can declare ordering on) + document the pattern +
  bounded timeout on oneshot startup (default 10min, per-app override) so a hung
  migration cannot wedge `cix up`.
- Failure semantics v0 = systemd semantics (dependent app does not start; `cix up`
  reports it). Half-run migrations are the app's transactional responsibility —
  stated loudly in docs (helm/k8s have the same gap, silently).
- **Deferred until the native shape proves insufficient**: abort-before-switch (old
  generation keeps serving on hook failure), post-switch smoke + auto-rollback.

## Scope & gate

cix-compose (schema + up sequencing); scenario: upgrade with passing hook applies;
upgrade with failing hook leaves old generation serving and exits nonzero; hook runs
against old db via the edge (assert observable order).

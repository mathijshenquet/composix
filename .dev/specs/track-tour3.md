# Track: tour3 — split the tour into scenario pages + add a run/ps scenario

You are extending the literate-tour harness at `crates/cix/tests/tour.rs` (read it and
`docs/tour.md` first; follow its established conventions: Doc builder, `sh` transcripts,
normalization, drift check, determinism, unique ports). Read `docs/design.md` D13 (degraded
--user mode) and D19. Where ambiguous, choose the boring option and note it in your LOG — do
NOT expand scope.

## Ground rules

- Work log: append `crates/cix/LOG.md`.
- Territory: `crates/cix/tests/`, `docs/` (tour output + index link), `README.md` (tour link
  only), dev-deps in `crates/cix/Cargo.toml`. Nothing else.
- COMMIT AS YOU GO; done gate includes `git status --short` clean.

## Deliverable 1: per-scenario pages

1. The generator now writes `docs/tour/` — one file per scenario, numbered for order
   (`01-tagging.md`, `02-moving.md`, …), plus `docs/tour/index.md`: intro (reuse the current
   header/auto-generated notice) + a linked list of scenarios with one-line descriptions.
2. Each scenario page: title, the auto-generated notice, its prose+transcripts, and a
   prev/next footer link. Links must work on the Jekyll site (link to `.html`).
3. Delete `docs/tour.md`; update the tour links in `docs/index.md` and `README.md` to the new
   index page.
4. Drift check and determinism test now cover ALL generated files (a removed/renamed scenario
   must fail drift until regenerated).

## Deliverable 2: "Running a service" scenario (new, last)

1. Build a tiny spec'd store item in the harness (same technique as the existing fixtures /
   `crates/cix-run/tests/user_run.rs`: generated dir with `cix-spec.json` + small shell
   service, `nix store add-path`) — cixSpec v2 is current; keep the fixture minimal (exec +
   one state dir).
2. Transcript: `cix run <path> --detach --user` — the LOUD degraded-mode warnings stay in the
   shown output on purpose; prose explains: `--user` is the rootless dev mode, the product
   target is the system manager with DynamicUser + full hardening (reference the design doc),
   and the VM check covers the system path.
3. `cix ps` showing the unit; then stop it (`systemctl --user stop …`), show it gone.
4. New normalizations needed: the unit-name nonce (`cix-run-<svc>-<hex>` → fixed), any
   warning lines that contain host-specific diagnostics (keep the warning, normalize
   variable parts). Scenario must be robust on a host without root and clean up its unit
   even on assertion failure.

## Done gate

`cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
green ×3 consecutively; generated pages read well and interlink; committed; LOG final
summary.

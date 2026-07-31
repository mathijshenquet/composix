# track/dirnames — micro-round: LOGS→LOGSDIR, CONFIG→CONFIGDIR

Read AGENTS.md first. Authoritative: design.md D52 addendum (resolved
2026-07-31): the role-dir directive family completes its systemd mirror —
`LOGSDIR` (LogsDirectory=), `CONFIGDIR` (ConfigurationDirectory=), joining
STATEDIR/CACHEDIR/RUNDIR. Scope: crates/cix-cixfile, examples, docs, tour.

1. Hard flip, exactly like STATE→STATEDIR in the previous round: `LOGS` and
   `CONFIG` become migration-grade parse errors naming the new spelling and
   D52; `LOGSDIR`/`CONFIGDIR` take over with identical semantics. Directive
   spelling only — manifest role keys (`dirs.logs`, `dirs.config`) unchanged.
2. Sweep every active Cixfile (examples/), docs (cixfile.md, migrate.md,
   docker.md where mentioned), and regenerate the tour.
3. Gate: `devenv shell -- cargo fmt --all --check`; warning-denied workspace
   all-target clippy; `cargo test --workspace`; tour regen + drift +
   determinism twice; `nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`.
   Exact repros + cleanup (reset test-created user units) in
   crates/cix-cixfile/LOG.md. Commit your work on this branch when green.

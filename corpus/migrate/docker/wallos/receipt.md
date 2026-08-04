# wallos migration receipt

Source revision: `3a7f965d0412b40ca29a678c90f0c830bc7e3faa` (2026-07-31).

Docker: `./check.sh docker` passed on 2026-07-31. Image:
`sha256:103ffa469b3455ebb5609802c124b6d7202bdd8606690c6e09ae18bc605eae46`.

Cix: `./check.sh cix` passed synchronously on 2026-08-02 after the file-first
authoring rewrite. Final item remained byte-identical:
`/nix/store/rds1fgd05lf8hh46i7h67inc2kxyw28c-cix-item-wallos`; the active unit
passed the bounded `/health.php` probe. The setup hook
created/migrated SQLite, nginx/PHP-FPM communicate over the declared runtime
directory, supercronic loaded the rewritten upstream schedule, and `/health.php`
returned exact `OK` after the host's D36 fallback.

The Cixfile uses D70's project-local `FROM … OVERLAY ./php.nix AS pkgs` form.
The three-line overlay composes `calendar`, `gd`, `intl`, `pdo_sqlite`, and
`zip` into `php`; its ordered content hash joins the locked nixpkgs pin in the
build and development-environment identities. This case no longer needs D4's
escape hatch. D4 remains the honest route for an organisation-owned full
universe tree or Nix computation that cannot be expressed as an overlay file.

The runtime retains nginx, PHP-FPM, scheduled jobs (via unprivileged supercronic),
durable SQLite/logo state, startup migrations, and outbound update behavior. The
checked Cixfile has not yet translated the endpoint into a CIP-79 `READINESS`
declaration; the receipt's bounded probe is therefore empirical, not a declared
startup contract.

The setup and supervisor programs are ordinary sibling files copied into the
artifact. The application rewrite is split into six readable RUN steps; the two
configuration heredocs remain together only because nginx configuration needs
`${pkgs.nginx}` interpolation, pending the unadopted FILE … FROM draft.

Overlay experience: package customization stays a concise native Nix function,
while the service contract, roles, readiness, setup, and process supervision
remain typed Cixfile declarations.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker warm evidence: the staged ordinary build and supplied `/health.php`
probe exited 0 (`PASS cix`), producing
`/nix/store/gxv88a6f0q2jp469pb8590c39fif13lr-cix-item-wallos`.

After `bash corpus/migrate/docker/fetch.sh wallos` exited 0, the assembler observed:

- `target/debug/cix build corpus/migrate/docker/wallos` exited 0 with
  `/nix/store/c23fh8898v075npv1f3paxpw596s13fa-cix-item-wallos`.
- `CIX=/home/mathijs/worktrees/composix/track-regen2/target/debug/cix
  ./check.sh cix` exited 0 synchronously from the case directory; the bounded
  probe returned exact `OK` and printed `PASS cix`.
- `target/debug/cix build --cold corpus/migrate/docker/wallos` exited 0 with the same
  item after executing the assembly RUN from a cold workspace.

Docker mode was not rerun.

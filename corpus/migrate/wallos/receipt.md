# wallos migration receipt

Source revision: `3a7f965d0412b40ca29a678c90f0c830bc7e3faa` (2026-07-31).

Docker: `./check.sh docker` passed on 2026-07-31. Image:
`sha256:103ffa469b3455ebb5609802c124b6d7202bdd8606690c6e09ae18bc605eae46`.

Cix: `./check.sh cix` passed. Final item:
`/nix/store/gm8yx7vsm3izqvfn94ji8jmqndggrm0r-cix-item-wallos`. The setup hook
created/migrated SQLite, nginx/PHP-FPM communicate over the declared runtime
directory, supercronic loaded the rewritten upstream schedule, and `/health.php`
returned exact `OK` after the host's D36 fallback.

This pair intentionally uses D4's `default.nix`
escape hatch: `php.withExtensions` composes `calendar`, `gd`, `intl`,
`pdo_sqlite`, and `zip`, which is evaluator-level package customization excluded
from Cixfile by D32. The Nix expression pins nixpkgs by revision and NAR hash.

The runtime retains nginx, PHP-FPM, scheduled jobs (via unprivileged supercronic),
durable SQLite/logo state, startup migrations, and outbound update behavior. Cix
still cannot encode the upstream Docker `HEALTHCHECK` as a D48 health edge.

Escape-hatch experience: the PHP extension expression itself is concise and
natural Nix. The cost is that leaving Cixfile also means manually assembling the
v5 manifest, app-path mounts, startup/setup scripts, and nixpkgs content pin.

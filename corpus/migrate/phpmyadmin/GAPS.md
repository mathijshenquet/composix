Generated: migrate.md@dd2f39a · terra · 2026-07-30
Status: current

- A login page passes, but the conversion drops upstream's PHP extensions, opcache/session/misc INI settings, upload limits, timezone, and upload-progress module. → case
- `config.inc.php`, `helpers.php`, and the entrypoint's runtime configuration generation are absent, so configuration parity is untested and largely unimplemented. → case
- `/var/www/html`, `/etc/phpmyadmin`, `/sessions`, and `/var/www/html/tmp` were replaced by `/srv/phpmyadmin` plus an unused `/var/lib/phpmyadmin` state role without a stated layout rationale. → prompt
- PHP is exposed through an implicit `/bin` link rather than an explicit artifact tool declaration. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- The receipt has no completed Docker transcript; the probe establishes page reachability only and must not be read as config parity. → evidence

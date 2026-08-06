Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: stale — regenerate with self-write trace hygiene

- The faithful 2.4.68 build and HTTP `It works!` probe pass. nixpkgs' APR/APR-util lacks LDAP support, and the configured source build does not provide several Docker-adjacent modules (`deflate`, `xml2enc`, `proxy_html`, `socache_dc`, `md`, `privileges`, `systemd`); none is claimed by the probe. → case
- `/proc/self/fd/1`, `/proc/self/fd/2`, `/dev/stdout`, and `/dev/stderr` are unavailable to Apache in the systemd sandbox. The service writes file logs through declared `LOGDIR /var/log/httpd` instead of Docker's fd symlinks. → prompt
- A stale GPG agent socket in the persistent workspace is not snapshot-safe; the FETCH clears `.gnupg`, and a pre-existing stale socket had to be moved aside before the unqualified build could run. → evidence
- The 2026-08-05 faithful cold replay exited 1 at the generated `output/usr/local/apache2/conf/extra/sedAsylf1` path. That exhibit matches the same-step-created random-path class fixed by the Valkey cold-trace track; regenerate before retaining or clearing the language finding. → language (CIP-87)
- The dissolved twin deliberately uses nixpkgs' Apache HTTPD rather than the faithful source build. → evidence

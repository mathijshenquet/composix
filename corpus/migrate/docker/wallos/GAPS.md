Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- The application webroot is `/app` rather than upstream `/var/www` because the latter collides with Cix's artifact mount and cannot contain the declared state descendants. → language (artifact-root/role-path collision)
- The wrapper still starts and supervises PHP-FPM and nginx inside one service; splitting those processes into compose members remains unattempted. → case
- Upstream's `dcron` service and `/etc/cron.d/cronjobs` schedule are not activated, so only the startup maintenance jobs run. → case
- Nginx/PHP-FPM file logs dissolve into journald, and the absent cron process means its file logs are absent too. → case
- Docker's `/health.php` check is exercised only by `check.sh`; the service does not declare the equivalent native HTTP `READINESS`. → case
- Port 18092 and its second nginx listener exist only because the supplied Cix probe hard-codes that host port without a runtime port override; port 80 remains the faithful listener. → evidence
